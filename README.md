# Discord Message Deleter

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/Discord-API_v10-7289DA.svg)](https://discord.com/developers/docs/intro)

A Rust tool to safely delete Discord messages in bulk, supporting both DMs and server channels.

[🇹🇷 Türkçe açıklama için tıklayın](#-discord-mesaj-silme-aracı)

> [!WARNING]  
> Using user tokens might be against Discord's Terms of Service. Use at your own risk.

## ✨ Features

- 🚀 Bulk message deletion with configurable delays
- 💬 Support for both DM and server channels
- 🛡️ Rate limit handling with automatic retry and backoff
- 📊 Detailed progress tracking and statistics
- ⚡ Skips system messages automatically
- 🕒 Handles messages older than 14 days
- ⚙️ Configurable deletion delays
- 🔄 Multi-threaded message listing and deletion
- ⏱️ Dynamic rate limit handling from Discord API
- 🔍 Channel/DM information display before deletion
- 🔁 Continuous search with user confirmation
- 📝 Detailed logging of operations

## 📋 Prerequisites

- 🦀 Rust (latest stable version)
- 🔑 Discord User Token
- 👤 Discord User ID
- 📝 Channel/DM ID

## 🔑 Getting Required Information

### 1. Enable Developer Mode
1. Open Discord Settings
2. Go to App Settings > Advanced
3. Enable Developer Mode

### 2. Get Your User Token
> [!CAUTION]  
> Never share your token with anyone! It gives full access to your account.

1. Open Discord in your web browser (discord.com)
2. Press F12 to open Developer Tools
3. Go to the Network tab
4. Click on any channel or perform any action
5. Look for a request to discord.com/api
6. Find the "authorization" header in the request headers
7. The value of this header is your user token

### 3. Get Your User ID
1. With Developer Mode enabled, right-click on your name anywhere in Discord
2. Click "Copy ID"
3. This is your User ID that you'll use as AUTHOR_ID

### 4. Get Channel/DM ID
1. For servers: Right-click on the channel name and click "Copy ID"
2. For DMs: Right-click on the DM chat and click "Copy Channel ID" (not "Copy ID")
3. This is your Channel ID that you'll use when running the program

## 🚀 Setup

1. Clone the repository:
```bash
git clone https://github.com/i358/discordMessageDeleter
cd discordMessageDeleter
```

2. Copy the example environment file:
```bash
cp .example.env .env
```

3. Edit `.env` file with your credentials:
```env
DISCORD_TOKEN=your_discord_token_here
AUTHOR_ID=your_user_id_here
```

> [!IMPORTANT]  
> Make sure your `.env` file contains valid credentials:
> - DISCORD_TOKEN should be your user token (starts with a specific format)
> - AUTHOR_ID should be your user ID (17-20 digit number)
> - Both fields are required and must be valid for the program to work

4. Build the project:
```bash
cargo build --release
```

## 📖 Usage

1. Run the program:
```bash
cargo run --release
```

2. Enter the requested information:
   - 📝 Channel ID: The ID of the channel/DM where you want to delete messages
   - ⏱️ Delete Delay: Time to wait between message deletions (in milliseconds)

3. Review the channel information:
   - Program will show channel type (DM/Server Channel)
   - For DMs: Shows the recipient's username
   - For Server Channels: Shows the channel name

4. Confirm the settings and let it run

## ⚙️ Advanced Features

- **Smart Batch Processing**: Processes messages in batches of 100
- **Continuous Search**: Asks to continue searching after empty batches
- **Rate Limit Handling**: Automatically adjusts timing based on Discord's rate limits
- **Progress Tracking**: Shows detailed statistics about deleted and failed messages
- **Channel Verification**: Validates channel access before starting
- **Error Recovery**: Handles various error scenarios gracefully

## 🛡️ Safety Features

- ✅ Channel information confirmation before starting
- 🔍 Input validation for all parameters
- 🚦 Automatic rate limit handling with backoff
- 📊 Detailed progress tracking
- ⚠️ Comprehensive error handling
- 🔄 Multi-threaded operation
- ⏱️ Operation time tracking

## ❗ Important Notes

> [!IMPORTANT]  
> 1. This tool is for educational purposes only
> 2. The tool can only delete messages sent by the account owner
> 3. System messages cannot be deleted
> 4. Messages older than 14 days will be deleted one by one
> 5. The program will ask for confirmation at various stages

## 🔒 Security

> [!TIP]  
> - Never share your Discord token
> - Keep your .env file secure
> - Don't commit sensitive information to git

## 🚀 Performance Tips

> [!NOTE]  
> - Adjust delays based on your needs
> - Default delete delay: 200ms
> - Rate limits are handled automatically
> - Multi-threading improves performance
> - Higher delays = More stable but slower
> - Lower delays = Faster but higher rate limit risk

## 📄 License

MIT License - See [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## 📬 Contact

For questions or suggestions:
1. Open an issue
2. Submit a pull request
3. Fork the project

---

# 🇹🇷 Discord Mesaj Silme Aracı

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/Discord-API_v10-7289DA.svg)](https://discord.com/developers/docs/intro)

Discord mesajlarınızı güvenli bir şekilde toplu olarak silmek için geliştirilmiş Rust tabanlı bir araç.

> [!WARNING]  
> Kullanıcı tokenlarının kullanımı Discord Kullanım Şartları'na aykırı olabilir. Riski size aittir.

## ✨ Özellikler

- 🚀 Ayarlanabilir gecikmelerle toplu mesaj silme
- 💬 Hem DM hem de sunucu kanalları desteği
- 🛡️ Otomatik yeniden deneme ve kademeli bekleme ile rate limit yönetimi
- 📊 Detaylı ilerleme takibi ve istatistikler
- ⚡ Sistem mesajlarını otomatik atlama
- 🕒 14 günden eski mesajları yönetebilme
- ⚙️ Ayarlanabilir silme gecikmeleri
- 🔄 Çok iş parçacıklı mesaj listeleme ve silme
- ⏱️ Discord API'den dinamik rate limit yönetimi
- 🔍 Silme işlemi öncesi kanal/DM bilgisi görüntüleme
- 🔁 Kullanıcı onaylı sürekli arama
- 📝 Detaylı işlem günlüğü

## 📋 Gereksinimler

- 🦀 Rust (en son kararlı sürüm)
- 🔑 Discord Kullanıcı Tokeni
- 👤 Discord Kullanıcı ID'si
- 📝 Kanal/DM ID'si

## 🔑 Gerekli Bilgileri Alma

### 1. Geliştirici Modunu Etkinleştirme
1. Discord Ayarlarını açın
2. Uygulama Ayarları > Gelişmiş'e gidin
3. Geliştirici Modunu etkinleştirin

### 2. Kullanıcı Tokeninizi Alma
> [!CAUTION]  
> Tokeninizi asla kimseyle paylaşmayın! Hesabınıza tam erişim sağlar.

1. Discord'u web tarayıcınızda açın (discord.com)
2. F12'ye basarak Geliştirici Araçlarını açın
3. Ağ (Network) sekmesine gidin
4. Herhangi bir kanala tıklayın veya bir işlem yapın
5. discord.com/api'ye yapılan bir istek bulun
6. İstek başlıklarında "authorization" başlığını bulun
7. Bu başlığın değeri sizin kullanıcı tokeninizdir

### 3. Kullanıcı ID'nizi Alma
1. Geliştirici Modu etkinken, Discord'da herhangi bir yerde adınıza sağ tıklayın
2. "ID'yi Kopyala"ya tıklayın
3. Bu, AUTHOR_ID olarak kullanacağınız Kullanıcı ID'nizdir

### 4. Kanal/DM ID'si Alma
1. Sunucular için: Kanal adına sağ tıklayın ve "ID'yi Kopyala"ya tıklayın
2. DM'ler için: DM sohbetine sağ tıklayın ve "Kanal ID'sini Kopyala"ya tıklayın ("ID'yi Kopyala" değil)
3. Bu, programı çalıştırırken kullanacağınız Kanal ID'sidir

## 🚀 Kurulum

1. Depoyu klonlayın:
```bash
git clone https://github.com/i358/discordMessageDeleter
cd discordMessageDeleter
```

2. Örnek çevre değişkenleri dosyasını kopyalayın:
```bash
cp .example.env .env
```

3. `.env` dosyasını bilgilerinizle düzenleyin:
```env
DISCORD_TOKEN=discord_tokeniniz
AUTHOR_ID=kullanici_id_niz
```

> [!IMPORTANT]  
> `.env` dosyanızın geçerli bilgiler içerdiğinden emin olun:
> - DISCORD_TOKEN kullanıcı tokeniniz olmalı (belirli bir formatla başlar)
> - AUTHOR_ID kullanıcı ID'niz olmalı (17-20 haneli sayı)
> - Her iki alan da gereklidir ve programın çalışması için geçerli olmalıdır

4. Projeyi derleyin:
```bash
cargo build --release
```

## 📖 Kullanım

1. Programı çalıştırın:
```bash
cargo run --release
```

2. İstenen bilgileri girin:
   - 📝 Kanal ID'si: Mesajları silmek istediğiniz kanal/DM'in ID'si
   - ⏱️ Silme Gecikmesi: Mesaj silmeleri arasında beklenecek süre (milisaniye)

3. Kanal bilgilerini inceleyin:
   - Program kanal tipini gösterecek (DM/Sunucu Kanalı)
   - DM'ler için: Alıcının kullanıcı adını gösterir
   - Sunucu Kanalları için: Kanal adını gösterir

4. Ayarları onaylayın ve çalıştırın

## ⚙️ Gelişmiş Özellikler

- **Akıllı Toplu İşleme**: Mesajları 100'lük gruplar halinde işler
- **Sürekli Arama**: Boş gruplardan sonra aramaya devam etmek için sorar
- **Rate Limit Yönetimi**: Discord'un rate limitlerine göre zamanlamayı otomatik ayarlar
- **İlerleme Takibi**: Silinen ve başarısız olan mesajlar hakkında detaylı istatistikler gösterir
- **Kanal Doğrulama**: Başlamadan önce kanal erişimini doğrular
- **Hata Kurtarma**: Çeşitli hata senaryolarını düzgün şekilde yönetir

## 🛡️ Güvenlik Özellikleri

- ✅ Başlamadan önce kanal bilgisi onayı
- 🔍 Tüm parametreler için girdi doğrulama
- 🚦 Kademeli bekleme ile otomatik rate limit yönetimi
- 📊 Detaylı ilerleme takibi
- ⚠️ Kapsamlı hata yönetimi
- 🔄 Çok iş parçacıklı çalışma
- ⏱️ İşlem süresi takibi

## ❗ Önemli Notlar

> [!IMPORTANT]  
> 1. Bu araç sadece eğitim amaçlıdır
> 2. Araç sadece hesap sahibinin gönderdiği mesajları silebilir
> 3. Sistem mesajları silinemez
> 4. 14 günden eski mesajlar tek tek silinecektir
> 5. Program çeşitli aşamalarda onay isteyecektir

## 🔒 Güvenlik

> [!TIP]  
> - Discord tokeninizi asla paylaşmayın
> - .env dosyanızı güvende tutun
> - Hassas bilgileri git'e commit etmeyin

## 🚀 Performans İpuçları

> [!NOTE]  
> - Gecikmeleri ihtiyacınıza göre ayarlayın
> - Varsayılan silme gecikmesi: 200ms
> - Rate limitler otomatik yönetilir
> - Çoklu iş parçacığı performansı artırır
> - Yüksek gecikmeler = Daha kararlı ama yavaş
> - Düşük gecikmeler = Daha hızlı ama rate limit riski yüksek

## 📄 Lisans

MIT Lisansı - Detaylar için [LICENSE](LICENSE) dosyasına bakın.

## 🤝 Katkıda Bulunma

Pull request'lere açığız. Büyük değişiklikler için lütfen önce bir issue açarak değişikliği tartışmaya açın.

## 📬 İletişim

Soru veya önerileriniz için:
1. Issue açın
2. Pull request gönderin
3. Projeyi forklayın

---

<div align="center">
Made with ❤️ by <a href="https://github.com/i358">i358</a>
</div>
