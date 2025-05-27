use teloxide::prelude::*;
use teloxide::types::InputFile;
// Removed: use teloxide::utils::command::BotCommands;

use scraper::{Html, Selector}; // Existing dependency
use reqwest::Url;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use log::{info, error, debug};

// Existing constants and structs
const BASE_URL: &str = "https://paste.compucalitv.lol/?v="; // Existing
const TEXTO_NO_DESEADO: &str = "DESCARGAS, PELICULAS Y SERIES"; // Existing

#[derive(Clone)] // Existing
struct ScanPattern {
    start_char: char,
    end_char: char,
    template: String,
}

// --- YTS Integration Structs ---
#[derive(Deserialize, Debug, Clone)]
struct YtsApiResponse {
    status: String,
    status_message: String,
    data: Option<YtsData>,
}

#[derive(Deserialize, Debug, Clone)]
struct YtsData {
    movies: Option<Vec<YtsMovie>>,
}

#[derive(Deserialize, Debug, Clone)]
struct YtsMovie {
    id: u32,
    title_long: String,
    year: u32,
    large_cover_image: String,
    torrents: Vec<YtsTorrent>,
    // Add other fields if needed, like `date_uploaded_unix` for more precise ordering
}

#[derive(Deserialize, Debug, Clone)]
struct YtsTorrent {
    url: String,
    hash: String,
    quality: String,
    // Removed: torrent_type: String, // "type" is a reserved keyword in Rust
    // Add other fields if needed
}

// --- Global State for YTS Monitoring ---
// Stores the ID of the latest movie processed globally by the bot
static LAST_PROCESSED_YTS_MOVIE_ID: Lazy<Arc<Mutex<Option<u32>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
// Stores chat IDs that are subscribed to YTS notifications
static SUBSCRIBED_CHAT_IDS: Lazy<Arc<Mutex<Vec<ChatId>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));
// Stores the handle of the single YTS monitoring task
static YTS_MONITOR_TASK_HANDLE: Lazy<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

const YTS_API_URL: &str = "https://yts.mx/api/v2/list_movies.json";
const YTS_CHECK_INTERVAL_SECONDS: u64 = 180; // Check every 2.5 minutes
const YTS_MOVIES_FETCH_LIMIT: u8 = 10; // Fetch up to 10 movies to check for new ones

// --- Helper Functions for YTS ---

/// Fetches the latest movies from the YTS API.
async fn fetch_latest_yts_movies(limit: u8) -> Result<Vec<YtsMovie>, reqwest::Error> {
    // API sorts by date_added desc by default
    let url = format!("{}?sort_by=date_added&order_by=desc&limit={}", YTS_API_URL, limit);
    debug!("Fetching YTS movies from URL: {}", url);
    let response = reqwest::get(&url).await?.json::<YtsApiResponse>().await?;

    if response.status == "ok" {
        if let Some(data) = response.data {
            if let Some(movies) = data.movies {
                return Ok(movies);
            }
        }
    }
    // If status is not "ok" or data/movies are missing, return empty or an error
    error!("Failed to fetch movies or parse response: Status: {}, Message: {}", response.status, response.status_message);
    Ok(Vec::new())
}

/// Constructs a magnet link for a given torrent hash and movie name.
fn construct_magnet_link(hash: &str, movie_name: &str) -> String {
    let encoded_movie_name = urlencoding::encode(movie_name);
    // Trackers recommended by YTS API documentation
    let trackers = [
        "udp://open.demonii.com:1337/announce",
        "udp://tracker.openbittorrent.com:80",
        "udp://tracker.coppersurfer.tk:6969",
        "udp://glotorrents.pw:6969/announce",
        "udp://tracker.opentrackr.org:1337/announce",
        "udp://torrent.gresille.org:80/announce",
        "udp://p4p.arenabg.com:1337",
        "udp://tracker.leechers-paradise.org:6969",
    ];
    let tracker_params = trackers.iter().map(|t| format!("&tr={}", urlencoding::encode(t))).collect::<String>();
    format!("`magnet:?xt=urn:btih:{}&dn={}{}`", hash, encoded_movie_name, tracker_params)
}

/// Sends a movie notification (photo with caption) to a specified chat.
async fn send_movie_notification(bot: &Bot, chat_id: ChatId, movie: &YtsMovie) -> ResponseResult<()> {
    if movie.torrents.is_empty() {
        info!("Movie '{}' (ID: {}) has no torrents, skipping notification.", movie.title_long, movie.id);
        return Ok(());
    }

    // Select a torrent (e.g., the first one, or filter by quality)
    let torrent = &movie.torrents[0]; // Taking the first available torrent

    let magnet_link = construct_magnet_link(&torrent.hash, &movie.title_long);
    let caption = format!(
        "{}\nAño: {}\nCalidad: {}\n\nMagnet: {}\n\nTorrent URL: {}",
        movie.title_long,
        movie.year,
        torrent.quality, // Added quality info
        magnet_link,
        torrent.url
    );

    if caption.len() > 1024 { // Telegram caption limit
        error!("Caption for movie {} is too long ({} chars). Sending truncated or alternative.", movie.title_long, caption.len());
        // Potentially send a shorter caption or multiple messages. For now, it might fail or be truncated by Telegram.
    }
    
    info!("Preparing to send notification for '{}' to chat {}", movie.title_long, chat_id);

    match Url::parse(&movie.large_cover_image) {
        Ok(img_url) => {
            match bot.send_photo(chat_id, InputFile::url(img_url)).caption(caption).await {
                Ok(_) => info!("Notification sent successfully for '{}' to chat {}", movie.title_long, chat_id),
                Err(e) => {
                    error!("Failed to send photo notification for '{}' to chat {}: {:?}. Trying text message.", movie.title_long, chat_id, e);
                    // Fallback to text message if photo send fails (e.g. image too big, bot blocked by user etc.)
                    let fallback_caption = format!(
                        "Nueva Película: {}\nAño: {}\nCalidad: {}\nCover: {}\nMagnet: {}\nTorrent URL: {}",
                        movie.title_long, movie.year, torrent.quality, movie.large_cover_image, magnet_link, torrent.url
                    );
                    bot.send_message(chat_id, fallback_caption).await?;
                }
            }
        }
        Err(e) => {
            error!("Invalid image URL '{}' for movie '{}': {:?}. Sending text message instead.", movie.large_cover_image, movie.title_long, e);
            let fallback_caption = format!(
                "Nueva Película: {}\nAño: {}\nCalidad: {}\nCover (URL inválida): {}\nMagnet: {}\nTorrent URL: {}",
                movie.title_long, movie.year, torrent.quality, movie.large_cover_image, magnet_link, torrent.url
            );
            bot.send_message(chat_id, fallback_caption).await?;
        }
    }
    Ok(())
}

// --- YTS Command Handlers ---

/// Command `/yts_init`: Subscribes the chat to YTS movie notifications and starts the monitor if not running.
async fn yts_init_command(bot: Bot, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    {
        let mut chats = SUBSCRIBED_CHAT_IDS.lock().await;
        if !chats.contains(&chat_id) {
            chats.push(chat_id);
            bot.send_message(chat_id, "✅ Te has suscrito a las notificaciones de nuevas películas de YTS.").await?;
            info!("Chat {} suscrito a notificaciones YTS.", chat_id);
        } else {
            bot.send_message(chat_id, "ℹ️ Ya estás suscrito a las notificaciones de YTS.").await?;
        }
    }

    // Ensure the global monitoring task is running
    let mut task_handle_guard = YTS_MONITOR_TASK_HANDLE.lock().await;
    if task_handle_guard.is_none() || task_handle_guard.as_ref().map_or(false, |h| h.is_finished()) {
        info!("Iniciando tarea de monitorización de YTS...");
        let bot_clone = bot.clone(); // Clone bot for the spawned task
        let last_id_global_clone = Arc::clone(&LAST_PROCESSED_YTS_MOVIE_ID);
        let subscribed_chats_global_clone = Arc::clone(&SUBSCRIBED_CHAT_IDS);

        let new_handle = tokio::spawn(async move {
            // Initial baseline setting: Set the last processed ID to the newest movie currently on YTS
            // This prevents spamming old movies on first run after a restart.
            {
                let mut last_id_lock = last_id_global_clone.lock().await;
                if last_id_lock.is_none() { // Only if no baseline exists at all (e.g. first ever run)
                    match fetch_latest_yts_movies(1).await {
                        Ok(movies) if !movies.is_empty() => {
                            *last_id_lock = Some(movies[0].id);
                            info!("Línea base inicial de YTS (LAST_PROCESSED_YTS_MOVIE_ID) establecida en ID: {}", movies[0].id);
                        }
                        Ok(_) => info!("No se encontraron películas para establecer la línea base inicial."),
                        Err(e) => error!("Error al obtener películas para la línea base inicial: {:?}", e),
                    }
                }
            }

            let mut interval = tokio::time::interval(Duration::from_secs(YTS_CHECK_INTERVAL_SECONDS));
            loop {
                interval.tick().await;
                debug!("Ejecutando comprobación periódica de YTS...");

                let mut movies_to_broadcast_this_tick: Vec<YtsMovie> = Vec::new();
                let mut new_highest_id_processed_this_tick = None;

                { // Scope for locking LAST_PROCESSED_YTS_MOVIE_ID
                    // Removed `mut` as per warning, guard itself is not mutated here.
                    let global_last_processed_id_guard = last_id_global_clone.lock().await;
                    let current_global_last_id = global_last_processed_id_guard.unwrap_or(0);

                    match fetch_latest_yts_movies(YTS_MOVIES_FETCH_LIMIT).await {
                        Ok(fetched_movies) => {
                            if fetched_movies.is_empty() {
                                debug!("No se encontraron películas en la comprobación periódica de YTS.");
                                continue; // Skip to next tick
                            }
                            
                            // Movies are fetched newest first. We want to find those newer than current_global_last_id.
                            let mut temp_new_movies = Vec::new();
                            for movie in fetched_movies.iter() { // Iterating newest to oldest
                                if movie.id > current_global_last_id {
                                    temp_new_movies.push(movie.clone());
                                } else {
                                    // Since fetched_movies is sorted (newest first),
                                    // once we hit a movie ID that's not newer, the rest won't be either.
                                    break;
                                }
                            }

                            if !temp_new_movies.is_empty() {
                                // The newest ID we are about to process and broadcast
                                new_highest_id_processed_this_tick = Some(temp_new_movies.first().unwrap().id);
                                // Reverse so that older new movies are broadcast first
                                movies_to_broadcast_this_tick = temp_new_movies.into_iter().rev().collect();
                            }
                        }
                        Err(e) => {
                            error!("Error buscando películas en YTS durante la comprobación periódica: {:?}", e);
                            continue; // Skip to next tick on error
                        }
                    }
                } // global_last_processed_id_guard is released here

                if !movies_to_broadcast_this_tick.is_empty() {
                    // Clone the list of subscribed chat IDs to avoid holding the lock while sending messages
                    let chat_ids_to_notify = subscribed_chats_global_clone.lock().await.clone();
                    if chat_ids_to_notify.is_empty() {
                        debug!("Hay nuevas películas pero ningún chat suscrito.");
                    }

                    for movie_to_broadcast in movies_to_broadcast_this_tick {
                        info!("Transmitiendo nueva película: '{}' (ID: {})", movie_to_broadcast.title_long, movie_to_broadcast.id);
                        for &target_chat_id in &chat_ids_to_notify {
                            if let Err(e) = send_movie_notification(&bot_clone, target_chat_id, &movie_to_broadcast).await {
                                error!("Error enviando notificación de '{}' a chat {}: {:?}", movie_to_broadcast.title_long, target_chat_id, e);
                                // Consider removing chat_id if bot is blocked or other persistent error
                            }
                        }
                    }

                    // After successfully processing and attempting to broadcast all new movies for this tick,
                    // update the global last processed ID.
                    if let Some(new_id) = new_highest_id_processed_this_tick {
                        let mut global_last_processed_id_guard = last_id_global_clone.lock().await;
                        // Ensure we are only moving forward
                        if new_id > global_last_processed_id_guard.unwrap_or(0) {
                           *global_last_processed_id_guard = Some(new_id);
                            info!("Global LAST_PROCESSED_YTS_MOVIE_ID actualizado a: {}", new_id);
                        }
                    }
                } else {
                    debug!("No hay nuevas películas de YTS para transmitir en este ciclo.");
                }
            }
        });
        *task_handle_guard = Some(new_handle);
        info!("Tarea de monitorización de YTS iniciada.");
    } else {
         info!("La tarea de monitorización de YTS ya está en ejecución.");
         // Optionally, notify the user again that they are subscribed and monitoring is active.
         bot.send_message(chat_id, "ℹ️ El monitor de YTS ya está activo y estás suscrito.").await?;
    }
    Ok(())
}

/// Command `/yts_stop`: Unsubscribes the chat from YTS movie notifications.
async fn yts_stop_command(bot: Bot, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let mut unsubscribed = false;
    {
        let mut chats = SUBSCRIBED_CHAT_IDS.lock().await;
        if let Some(pos) = chats.iter().position(|&id| id == chat_id) {
            chats.remove(pos);
            unsubscribed = true;
        }
    }

    if unsubscribed {
        bot.send_message(chat_id, "✅ Te has dado de baja de las notificaciones de YTS.").await?;
        info!("Chat {} dado de baja de notificaciones YTS.", chat_id);

        // Optional: Stop the monitor task if no chats are subscribed.
        // This might be desirable to save resources if the bot has other functions.
        let chats = SUBSCRIBED_CHAT_IDS.lock().await;
        if chats.is_empty() {
            info!("No hay chats suscritos. Intentando detener la tarea de monitorización de YTS...");
            let mut task_handle_guard = YTS_MONITOR_TASK_HANDLE.lock().await;
            if let Some(handle) = task_handle_guard.take() { // .take() removes the handle
                handle.abort(); // Abort the task
                info!("Tarea de monitorización de YTS detenida ya que no hay suscriptores.");
                 // Notify admin or log, don't message user as they just unsubscribed.
            }
        }
    } else {
        bot.send_message(chat_id, "ℹ️ No estabas suscrito a las notificaciones de YTS.").await?;
    }
    Ok(())
}


// --- Existing Functions (modified slightly for safety or clarity if needed) ---
fn parse_pattern(input: &str) -> Option<ScanPattern> { // Existing
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let parse_part = |s: &str| -> Option<(char, String)> {
        let start_idx = s.find('[')?;
        let end_idx = s.find(']')?;
        if start_idx >= end_idx { return None; } // Ensure valid indices
        let variable_char = s.get(start_idx + 1..end_idx)?.chars().next()?;
        let prefix = s.get(..start_idx)?.to_string();
        let suffix = s.get(end_idx + 1..)?.to_string();
        Some((variable_char, format!("{}{{}}{}", prefix, suffix)))
    };
    let (start_char, start_tmpl) = parse_part(parts[0])?;
    let (end_char, end_tmpl) = parse_part(parts[1])?;
    if start_tmpl != end_tmpl { // This check might be too restrictive if templates are meant to differ
        return None;
    }
    Some(ScanPattern {
        start_char,
        end_char,
        template: start_tmpl,
    })
}

async fn check_links(bot: Bot, msg: Message, pattern: String) -> ResponseResult<()> { // Existing
    let chat_id = msg.chat.id;
    let scan_pattern = match parse_pattern(&pattern) {
        Some(p) => p,
        None => {
            bot.send_message(chat_id, "⚠️ Formato de patrón inválido. Ejemplos:\n/check l[c]a-l[m]a\n/check [A]bc-[Z]bc")
                .await?;
            return Ok(());
        }
    };

    bot.send_message(chat_id, &format!("🔍 Escaneando con patrón: {}...", pattern)).await?;
    let start_byte = scan_pattern.start_char as u8;
    let end_byte = scan_pattern.end_char as u8;
    let (eff_start, eff_end) = if start_byte <= end_byte { (start_byte, end_byte) } else { (end_byte, start_byte) };

    for c_byte in eff_start..=eff_end {
        let current_char = c_byte as char;
        // Ensure char is valid before forming URL, especially if range is large
        if !current_char.is_alphanumeric() && !"[](){}".contains(current_char) { // Example filter, adjust as needed
             // debug!("Skipping non-alphanumeric char in pattern scan: {}", current_char);
             // continue;
        }

        let param_val = scan_pattern.template.replace("{}", &current_char.to_string());
        let url = format!("{}{}", BASE_URL, param_val);
        
        info!("Verificando URL (scraper): {}", url);
        
        match check_page(&url).await {
            Ok(Some(title)) => {
                if title != TEXTO_NO_DESEADO && !title.trim().is_empty() { // Added !title.trim().is_empty()
                    let message_text = format!("✅ ¡Encontrado!\nURL: {}\nTítulo: {}", url, title);
                    bot.send_message(chat_id, &message_text).await?;
                } else {
                    debug!("Título no deseado o vacío encontrado en {}: '{}'", url, title);
                }
            }
            Ok(None) => {
                debug!("No se encontró título en {}", url);
                continue;
            }
            Err(e) => {
                error!("Error al verificar la página {} (scraper): {}", url, e);
                // Optionally notify user about specific page error, or just log
                // bot.send_message(chat_id, format!("⚠️ Error al verificar {}", url)).await?;
                continue;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await; // Keep delay to be polite to server
    }
    bot.send_message(chat_id, "🚀 Escaneo (scraper) completado!").await?;
    Ok(())
}

async fn check_page(url: &str) -> Result<Option<String>, reqwest::Error> { // Existing
    let res = reqwest::get(url).await?;
    if !res.status().is_success() {
        error!("HTTP error {} for URL: {}", res.status(), url);
        return Ok(None); // Or return specific error
    }
    let html_content = res.text().await?;
    let document = Html::parse_document(&html_content);
    let title_selector = Selector::parse("title").unwrap(); // .unwrap() is okay if "title" is always valid selector

    Ok(document
        .select(&title_selector)
        .next()
        .map(|title_element| title_element.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())) // Ensure title is not just whitespace
}

async fn start_command(bot: Bot, msg: Message) -> ResponseResult<()> { // Renamed from `start` for clarity
    let help_text = "¡Bienvenido al Scraper Avanzado y Notificador YTS! 🕷️🎬\n\n\
Comandos disponibles:\n
/start - Muestra esta ayuda.
/check [patrón] - Inicia escaneo de links (función original). Ej: /check l[c]a-l[m]a
/yts_init - Suscribe este chat a notificaciones de nuevas películas de YTS.
/yts_stop - Da de baja este chat de las notificaciones de YTS.";
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Iniciando bot...");
    // Removed .auto_send() as it's deprecated and caused type mismatch.
    // Bot methods are now directly awaitable.
    let bot = Bot::from_env(); 

    // Define command handlers using dptree branches, similar to the original structure
    let handler = Update::filter_message()
        .branch( // /start command
            dptree::entry()
                .filter_map(|msg: Message| msg.text().map(ToOwned::to_owned))
                .filter(|text: String| text == "/start")
                .endpoint(start_command)
        )
        .branch( // /check command
            dptree::entry()
                .filter_map(|msg: Message| msg.text().map(ToOwned::to_owned))
                .filter(|text: String| text.starts_with("/check "))
                .endpoint(|bot: Bot, msg: Message, text: String| async move {
                    let pattern = text.trim_start_matches("/check ").trim().to_string();
                    if pattern.is_empty() {
                        bot.send_message(msg.chat.id, "⚠️ Por favor, proporciona un patrón después de /check. Ejemplo: /check l[c]a-l[m]a").await?;
                        return Ok(());
                    }
                    check_links(bot, msg, pattern).await
                })
        )
        .branch( // /yts_init command
            dptree::entry()
                .filter_map(|msg: Message| msg.text().map(ToOwned::to_owned))
                .filter(|text: String| text == "/yts_init")
                .endpoint(yts_init_command)
        )
        .branch( // /yts_stop command
            dptree::entry()
                .filter_map(|msg: Message| msg.text().map(ToOwned::to_owned))
                .filter(|text: String| text == "/yts_stop")
                .endpoint(yts_stop_command)
        );
        // Add a default handler for unrecognised commands or text if desired
        // .branch(dptree::endpoint(|msg: Message, bot: Bot| async move {
        //     bot.send_message(msg.chat.id, "Comando no reconocido.").await?;
        //     Ok(())
        // }));


    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    
    info!("Bot detenido.");
}