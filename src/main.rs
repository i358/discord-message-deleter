use anyhow::{Context, Result, anyhow};
use dotenv::dotenv;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;
use std::{env, time::{Duration, Instant}, io::{self, Write}, sync::{Arc, Mutex}};
use tokio::{time::sleep, sync::mpsc};

const DISCORD_API: &str = "https://discord.com/api/v10";
const MESSAGES_PER_REQUEST: u32 = 100;
const MIN_DELETE_DELAY: u64 = 50;
const MAX_DELETE_DELAY: u64 = 5000;

#[derive(Debug, Deserialize, Clone)]
struct Message {
    id: String,
    author: Author,
}

#[derive(Debug, Deserialize, Clone)]
struct Author {
    id: String,
}

#[derive(Debug, Deserialize)]
struct RateLimitResponse {
    retry_after: f64,
}

#[derive(Debug, Deserialize)]
struct ChannelInfo {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    channel_type: u8,
    recipients: Option<Vec<User>>,
}

#[derive(Debug, Deserialize)]
struct User {
    username: String,
    discriminator: String,
}

struct Stats {
    total_deleted: usize,
    total_failed: usize,
    start_time: Instant,
    messages_in_process: usize,
}

struct DiscordClient {
    client: Client,
    channel_id: String,
    author_id: String,
    stats: Arc<Mutex<Stats>>,
    delete_delay: u64,
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

    fn new(token: String, channel_id: String, author_id: String, delete_delay: u64) -> Result<Self> {
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
            stats: Arc::new(Mutex::new(Stats {
                total_deleted: 0,
                total_failed: 0,
                start_time: Instant::now(),
                messages_in_process: 0,
            })),
            delete_delay,
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
                
                return Ok(all_messages);
            } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let rate_limit: RateLimitResponse = response.json().await?;
                println!("Rate limited! Waiting {} seconds...", rate_limit.retry_after);
                sleep(Duration::from_secs_f64(rate_limit.retry_after)).await;
                continue;
            } else {
                let status = response.status();
                let text = response.text().await?;
                println!("API Error URL: {}", url);
                anyhow::bail!("Failed to get messages: {} - {}", status, text);
            }
        }
    }

    async fn delete_message(&self, message_id: &str) -> Result<()> {
        let mut backoff = 1.0; 
        let max_backoff = 30.0; 
        
        let url = format!(
            "{}/channels/{}/messages/{}",
            DISCORD_API, self.channel_id, message_id
        );
    
        loop {
            let response = self.client.delete(&url).send().await?;
            let status = response.status();
            
            if status.is_success() {
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.total_deleted += 1;
                }
                println!("Deleted message {}", message_id);
                return Ok(());
            } else if status == StatusCode::TOO_MANY_REQUESTS {
                let rate_limit: RateLimitResponse = response.json().await?;
                let wait_time = f64::max(rate_limit.retry_after, backoff);
                println!("Rate limited! Waiting {} seconds before retrying...", wait_time);
                sleep(Duration::from_secs_f64(wait_time)).await;
                backoff = f64::min(backoff * 2.0, max_backoff);
                continue;
            } else if status == StatusCode::NOT_FOUND {
                println!("Message {} not found (already deleted or too old)", message_id);
                return Ok(());
            } else if status == StatusCode::FORBIDDEN {
                println!("No permission to delete message {}", message_id);
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.total_failed += 1;
                }
                return Ok(());
            } else {
                let text = response.text().await?;
                println!("Error deleting message {}: {} - {}", message_id, status, text);
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.total_failed += 1;
                }
                
                if status.is_server_error() {
                    println!("Server error, retrying after backoff...");
                    sleep(Duration::from_secs_f64(backoff)).await;
                    backoff = f64::min(backoff * 2.0, max_backoff);
                    continue;
                }
                
                return Ok(());
            }
        }
    }

    async fn process_messages(&self, rx: mpsc::Receiver<Message>) {
        let mut rx = rx;
        
        while let Some(message) = rx.recv().await {
            match self.delete_message(&message.id).await {
                Ok(_) => (),
                Err(e) => {
                    println!("Error deleting message {}: {}", message.id, e);
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.total_failed += 1;
                        stats.messages_in_process = stats.messages_in_process.saturating_sub(1);
                    }
                }
            }
            
            {
                let mut stats = self.stats.lock().unwrap();
                stats.messages_in_process = stats.messages_in_process.saturating_sub(1);
            }
            
            sleep(Duration::from_millis(self.delete_delay)).await;
        }
    }

    async fn list_messages(&self, tx: mpsc::Sender<Message>) -> Result<()> {
        let mut last_message_id: Option<String> = None;
        let mut seen_message_ids = std::collections::HashSet::new();
        let mut total_batches = 0;
        let mut consecutive_empty = 0;
        let mut total_found = 0;

        loop {
            loop {
                let messages_in_process = {
                    let stats = self.stats.lock().unwrap();
                    stats.messages_in_process
                };
                
                if messages_in_process == 0 {
                    break;
                }
                
                sleep(Duration::from_millis(500)).await;
            }

            let all_messages = self.get_messages(last_message_id.as_deref()).await?;
            total_batches += 1;
            
            if all_messages.is_empty() {
                consecutive_empty += 1;
                println!("\nEmpty Batch #{} (Attempt {} of 10)", total_batches, consecutive_empty);
                
                if consecutive_empty >= 10 {
                    println!("\nNo messages found in the last 10 batches.");
                    println!("Last checked message ID: {}", last_message_id.as_deref().unwrap_or("None"));
                    println!("Would you like to continue searching older messages? (Y/n): ");
                    if read_input("")?.to_lowercase() == "n" {
                        println!("Search stopped by user.");
                        break;
                    } else {
                        consecutive_empty = 0;
                        println!("Continuing search...");
                        continue;
                    }
                }

                if last_message_id.is_some() {
                    println!("Moving to older messages (before ID: {})...", last_message_id.as_ref().unwrap());
                    sleep(Duration::from_millis(200)).await;
                    continue;
                } else {
                    println!("No messages found and no message ID to paginate from. Stopping.");
                    break;
                }
            }

            if let Some(last) = all_messages.last() {
                last_message_id = Some(last.id.clone());
            }

            let user_messages: Vec<_> = all_messages.into_iter()
                .filter(|m| m.author.id == self.author_id)
                .collect();

            let batch_user_messages = user_messages.len();
            total_found += batch_user_messages;

            println!("\nBatch Information:");
            println!("Batch #{}", total_batches);
            println!("Found {} user messages to delete (Total found: {})", batch_user_messages, total_found);

            if batch_user_messages > 0 {
                consecutive_empty = 0;
                
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.messages_in_process = batch_user_messages;
                }

                for message in user_messages {
                    if seen_message_ids.contains(&message.id) {
                        println!("Skipping already processed message: {}", message.id);
                        continue;
                    }

                    seen_message_ids.insert(message.id.to_string());
                    if tx.send(message).await.is_err() {
                        println!("Receiver has been dropped, stopping message listing");
                        return Ok(());
                    }
                }
            } else {
                consecutive_empty += 1;
                println!("Empty Batch #{} (Attempt {} of 10)", total_batches, consecutive_empty);
                
                if total_batches == 1 {
                    println!("\nNo messages found from you in the first 100 messages.");
                    println!("Would you like to search older messages? This might take longer.");
                    if read_input("Continue searching? (Y/n): ")?.to_lowercase() == "n" {
                        println!("Search aborted by user.");
                        break;
                    }
                    consecutive_empty = 0;
                    continue;
                }
                
                if consecutive_empty >= 10 {
                    println!("\nNo user messages found in the last 10 batches.");
                    println!("Last checked message ID: {}", last_message_id.as_deref().unwrap_or("None"));
                    println!("Would you like to continue searching older messages? (Y/n): ");
                    if read_input("")?.to_lowercase() == "n" {
                        println!("Search stopped by user.");
                        break;
                    } else {
                        consecutive_empty = 0;
                        println!("Continuing search...");
                        continue;
                    }
                }
            }
            
            {
                let stats = self.stats.lock().unwrap();
                println!(
                    "\nOverall Progress:");
                println!("Successfully deleted: {}", stats.total_deleted);
                println!("Failed to delete: {}", stats.total_failed);
                println!("Messages in process: {}", stats.messages_in_process);
                println!("Total batches checked: {}", total_batches);
                println!("Total messages found: {}", total_found);
                
                if stats.total_deleted + stats.total_failed >= total_found && 
                   consecutive_empty >= 10 && 
                   total_found > 0 {
                    println!("\nAll found messages have been processed.");
                    println!("Last checked message ID: {}", last_message_id.as_deref().unwrap_or("None"));
                    println!("Would you like to continue searching older messages? (Y/n): ");
                    if read_input("")?.to_lowercase() == "n" {
                        println!("Search stopped by user.");
                        break;
                    } else {
                        consecutive_empty = 0;
                        println!("Continuing search...");
                    }
                }
            }
            
            sleep(Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn delete_all_messages(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel(100);
        
        let list_client = self.clone();
        let process_client = self.clone();
        
        println!("\nStarting message search and deletion process...");
        println!("This may take a while depending on the number of messages and rate limits.");
        println!("The program will automatically stop when all messages are processed.");
        
        let list_handle = tokio::spawn(async move {
            list_client.list_messages(tx).await
        });
        
        let process_handle = tokio::spawn(async move {
            process_client.process_messages(rx).await
        });
        
        list_handle.await??;
        process_handle.await?;
        
        let stats = self.stats.lock().unwrap();
        let elapsed = stats.start_time.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        
        println!("\nOperation Complete!");
        println!("Successfully deleted: {}", stats.total_deleted);
        println!("Failed to delete: {}", stats.total_failed);
        println!("Total time elapsed: {}m {}s", minutes, seconds);
        
        Ok(())
    }

    async fn get_channel_info(token: &str, channel_id: &str) -> Result<ChannelInfo> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(token)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        let response = client.get(&format!("{}/channels/{}", DISCORD_API, channel_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow!("Failed to get channel info"))
        }
    }
}

impl Clone for DiscordClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            channel_id: self.channel_id.clone(),
            author_id: self.author_id.clone(),
            stats: Arc::clone(&self.stats),
            delete_delay: self.delete_delay,
        }
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
                // * Get channel info
                match DiscordClient::get_channel_info(&token, &input).await {
                    Ok(info) => {
                        println!("\nChannel Information:");
                        println!("------------------");
                        match info.channel_type {
                            1 => {
                                println!("Type: Direct Message (DM)");
                                if let Some(recipients) = info.recipients {
                                    for user in recipients {
                                        println!("With User: {}#{}", user.username, user.discriminator);
                                    }
                                }
                            },
                            3 => println!("Type: Group DM"),
                            0 | 2 | 4 | 5 | 6 => {
                                println!("Type: Server Channel");
                                if let Some(name) = info.name {
                                    println!("Channel Name: #{}", name);
                                }
                            },
                            _ => println!("Type: Unknown Channel Type"),
                        }
                        println!("------------------");
                        break input;
                    },
                    Err(e) => {
                        println!("Warning: Could not get channel details: {}", e);
                        println!("Do you want to continue anyway? (y/N): ");
                        if read_input("")?.to_lowercase() == "y" {
                            break input;
                        } else {
                            continue;
                        }
                    }
                }
            } else {
                println!("Channel not found or no access. Please try again.");
                continue;
            }
        }
        println!("Invalid channel ID format. Please enter a valid Discord ID.");
    };

    let delete_delay = read_number_input(
        &format!("Enter delay between message deletions ({}ms-{}ms, default 200ms): ",
            MIN_DELETE_DELAY, MAX_DELETE_DELAY),
        MIN_DELETE_DELAY,
        MAX_DELETE_DELAY,
        200,
    )?;

    println!("\nConfiguration:");
    println!("Channel ID: {}", channel_id);
    println!("Delete Delay: {}ms", delete_delay);
    println!("Author ID: {}", author_id);
    
    if read_input("\nContinue? (Y/n): ")?.to_lowercase() != "n" {
        println!("\nStarting message deletion process...");
        let discord = DiscordClient::new(token, channel_id, author_id, delete_delay)?;
        discord.delete_all_messages().await?;
    } else {
        println!("Operation aborted by user.");
    }

    Ok(())
} 