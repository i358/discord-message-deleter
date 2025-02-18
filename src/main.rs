use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;
use std::{env, time::Duration, io::{self, Write}};
use tokio::time::sleep;

const DISCORD_API: &str = "https://discord.com/api/v10";
const MESSAGES_PER_REQUEST: u32 = 100;
const MIN_DELETE_DELAY: u64 = 50;
const MAX_DELETE_DELAY: u64 = 5000;
const MIN_RATE_LIMIT_DELAY: u64 = 5;
const MAX_RATE_LIMIT_DELAY: u64 = 60;

#[derive(Debug, Deserialize)]
struct Message {
    id: String,
    author: Author,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct Author {
    id: String,
}

#[derive(Debug, Deserialize)]
struct RateLimitResponse {}

struct DiscordClient {
    client: Client,
    channel_id: String,
    author_id: String,
    total_deleted: usize,
    total_failed: usize,
    delete_delay: u64,
    rate_limit_delay: u64,
}

impl DiscordClient {
    async fn validate_token(token: &str) -> Result<bool> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(token)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        let response = client.get("https://discord.com/api/v10/users/@me")
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn validate_channel(token: &str, channel_id: &str) -> Result<bool> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(token)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        let response = client.get(&format!("https://discord.com/api/v10/channels/{}", channel_id))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    fn new(token: String, channel_id: String, author_id: String, delete_delay: u64, rate_limit_delay: u64) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&token)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            channel_id,
            author_id,
            total_deleted: 0,
            total_failed: 0,
            delete_delay,
            rate_limit_delay,
        })
    }

    async fn get_messages(&self, before: Option<&str>) -> Result<Vec<Message>> {
        let mut url = format!(
            "{}/channels/{}/messages?limit={}",
            DISCORD_API, self.channel_id, MESSAGES_PER_REQUEST
        );

        if let Some(message_id) = before {
            url.push_str(&format!("&before={}", message_id));
        }

        println!("Fetching messages from URL: {}", url);

        loop {
            let response = self.client.get(&url).send().await?;
            
            if response.status().is_success() {
                let all_messages: Vec<Message> = response.json().await?;
                println!("Total messages received from API: {}", all_messages.len());
                
                if !all_messages.is_empty() {
                    if let Some(first) = all_messages.first() {
                        println!("Oldest message in batch: {} (ID: {})", first.timestamp, first.id);
                    }
                    if let Some(last) = all_messages.last() {
                        println!("Newest message in batch: {} (ID: {})", last.timestamp, last.id);
                    }
                }
                
                let filtered: Vec<Message> = all_messages.into_iter()
                    .filter(|m| m.author.id == self.author_id)
                    .collect();
                
                println!("Messages after filtering for user: {}", filtered.len());
                return Ok(filtered);
            } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let _: RateLimitResponse = response.json().await?;
                println!("Rate limited! Waiting {} seconds...", self.rate_limit_delay);
                sleep(Duration::from_secs(self.rate_limit_delay)).await;
                continue;
            } else {
                let status = response.status();
                let text = response.text().await?;
                println!("API Error URL: {}", url);
                anyhow::bail!("Failed to get messages: {} - {}", status, text);
            }
        }
    }

    async fn delete_message(&mut self, message_id: &str) -> Result<()> {
        let url = format!(
            "{}/channels/{}/messages/{}",
            DISCORD_API, self.channel_id, message_id
        );

        loop {
            let response = self.client.delete(&url).send().await?;
            let status = response.status();
            
            if status.is_success() {
                self.total_deleted += 1;
                println!("Deleted message {}", message_id);
                return Ok(());
            } else if status == StatusCode::TOO_MANY_REQUESTS {
                let _: RateLimitResponse = response.json().await?;
                println!("Rate limited! Waiting {} seconds...", self.rate_limit_delay);
                sleep(Duration::from_secs(self.rate_limit_delay)).await;
                continue;
            } else if status == StatusCode::FORBIDDEN {
                let error_text = response.text().await?;
                if error_text.contains("50021") || error_text.contains("system message") {
                    println!("Skipping system message {}", message_id);
                    return Ok(());
                }
                self.total_failed += 1;
                anyhow::bail!("Failed to delete message: {} - {}", status, error_text);
            } else if status == StatusCode::NOT_FOUND {
                println!("Message {} not found, skipping", message_id);
                self.total_failed += 1;
                return Ok(());
            } else {
                let status = response.status();
                let text = response.text().await?;
                self.total_failed += 1;
                anyhow::bail!("Failed to delete message: {} - {}", status, text);
            }
        }
    }

    async fn delete_all_messages(&mut self) -> Result<()> {
        let mut last_message_id: Option<String> = None;
        let mut total_messages = 0;
        let mut consecutive_empty = 0;
        let mut seen_message_ids = std::collections::HashSet::new();

        loop {
            let messages = self.get_messages(last_message_id.as_deref()).await?;
            
            if messages.is_empty() {
                let url = format!(
                    "{}/channels/{}/messages?limit={}{}",
                    DISCORD_API, 
                    self.channel_id, 
                    MESSAGES_PER_REQUEST,
                    last_message_id.as_ref().map_or(String::new(), |id| format!("&before={}", id))
                );
                
                let response = self.client.get(&url).send().await?;
                if response.status().is_success() {
                    let all_messages: Vec<Message> = response.json().await?;
                    if !all_messages.is_empty() {
                        if let Some(last) = all_messages.last() {
                            println!("No user messages found, skipping to before: {} ({})", last.timestamp, last.id);
                            last_message_id = Some(last.id.clone());
                            consecutive_empty = 0;
                            continue;
                        }
                    }
                }
                
                consecutive_empty += 1;
                println!("No messages found in this batch. Attempt {} of 3", consecutive_empty);
                
                if consecutive_empty >= 3 {
                    println!("No more messages found after 3 attempts. Stopping.");
                    break;
                }
                
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            consecutive_empty = 0;
            total_messages += messages.len();
            
            println!("\nBatch Information:");
            println!("Found {} messages to delete in this batch", messages.len());

            for message in &messages {
                if seen_message_ids.contains(&message.id) {
                    println!("Skipping already processed message: {}", message.id);
                    continue;
                }

                seen_message_ids.insert(message.id.to_string());
                match self.delete_message(&message.id).await {
                    Ok(_) => (),
                    Err(e) => println!("Error deleting message {}: {}", message.id, e),
                }
                sleep(Duration::from_millis(self.delete_delay)).await;
            }

            if let Some(last) = messages.last() {
                last_message_id = Some(last.id.clone());
            }
            
            println!(
                "\nOverall Progress:");
            println!("Total messages found: {}", total_messages);
            println!("Successfully deleted: {}", self.total_deleted);
            println!("Failed to delete: {}", self.total_failed);
            println!("Remaining: {}", total_messages - (self.total_deleted + self.total_failed));
        }

        println!("\nOperation Complete!");
        println!("Total messages processed: {}", total_messages);
        println!("Successfully deleted: {}", self.total_deleted);
        println!("Failed to delete: {}", self.total_failed);
        Ok(())
    }
}

fn validate_snowflake(id: &str) -> bool {
    id.len() >= 17 && id.len() <= 20 && id.chars().all(|c| c.is_ascii_digit())
}

fn read_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn read_number_input(prompt: &str, min: u64, max: u64, default: u64) -> Result<u64> {
    let input = read_input(prompt)?;
    if input.is_empty() {
        return Ok(default);
    }

    match input.parse::<u64>() {
        Ok(num) if num >= min && num <= max => Ok(num),
        Ok(_) => Err(anyhow!("Value must be between {} and {}", min, max)),
        Err(_) => Err(anyhow!("Invalid number format")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("Discord Message Deleter");
    println!("----------------------");

    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => token,
        Err(_) => return Err(anyhow!("DISCORD_TOKEN not found in .env file")),
    };

    if !DiscordClient::validate_token(&token).await? {
        return Err(anyhow!("Invalid Discord token"));
    }

    let author_id = match env::var("AUTHOR_ID") {
        Ok(id) if validate_snowflake(&id) => id,
        Ok(_) => return Err(anyhow!("Invalid AUTHOR_ID format in .env file")),
        Err(_) => return Err(anyhow!("AUTHOR_ID not found in .env file")),
    };

    let channel_id = loop {
        let input = read_input("Enter channel ID: ")?;
        if validate_snowflake(&input) {
            if DiscordClient::validate_channel(&token, &input).await? {
                break input;
            } else {
                println!("Channel not found or no access. Please try again.");
                continue;
            }
        }
        println!("Invalid channel ID format. Please enter a valid Discord ID.");
    };

    let delete_delay = read_number_input(
        &format!("Enter delay between message deletions ({}ms-{}ms, default 50ms): ",
            MIN_DELETE_DELAY, MAX_DELETE_DELAY),
        MIN_DELETE_DELAY,
        MAX_DELETE_DELAY,
        50,
    )?;

    let rate_limit_delay = read_number_input(
        &format!("Enter rate limit delay ({}s-{}s, default 5s): ",
            MIN_RATE_LIMIT_DELAY, MAX_RATE_LIMIT_DELAY),
        MIN_RATE_LIMIT_DELAY,
        MAX_RATE_LIMIT_DELAY,
        5,
    )?;

    println!("\nConfiguration:");
    println!("Channel ID: {}", channel_id);
    println!("Delete Delay: {}ms", delete_delay);
    println!("Rate Limit Delay: {}s", rate_limit_delay);
    println!("Author ID: {}", author_id);
    
    if read_input("\nContinue? (y/N): ")?.to_lowercase() != "y" {
        println!("Operation aborted by user.");
        return Ok(());
    }

    println!("\nStarting message deletion process...");

    let mut discord = DiscordClient::new(token, channel_id, author_id, delete_delay, rate_limit_delay)?;
    discord.delete_all_messages().await?;

    Ok(())
} 