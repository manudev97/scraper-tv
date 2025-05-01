use teloxide::prelude::*;
use scraper::{Html, Selector};
use std::time::Duration;
use log::{info, error};

const BASE_URL: &str = "https://paste.compucalitv.lol/?v=";
const TEXTO_NO_DESEADO: &str = "DESCARGAS, PELICULAS Y SERIES";

#[derive(Clone)]
struct ScanPattern {
    start_char: char,
    end_char: char,
    template: String,
}

fn parse_pattern(input: &str) -> Option<ScanPattern> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let parse_part = |s: &str| -> Option<(char, String)> {
        let start_idx = s.find('[')?;
        let end_idx = s.find(']')?;
        let variable_char = s[start_idx+1..end_idx].chars().next()?;
        let prefix = s[..start_idx].to_string();
        let suffix = s[end_idx+1..].to_string();
        Some((variable_char, format!("{}{{}}{}", prefix, suffix)))
    };

    let (start_char, start_tmpl) = parse_part(parts[0])?;
    let (end_char, end_tmpl) = parse_part(parts[1])?;

    if start_tmpl != end_tmpl {
        return None;
    }

    Some(ScanPattern {
        start_char,
        end_char,
        template: start_tmpl,
    })
}

async fn check_links(bot: Bot, msg: Message, pattern: String) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    
    let scan_pattern = match parse_pattern(&pattern) {
        Some(p) => p,
        None => {
            bot.send_message(chat_id, "⚠️ Formato inválido. Ejemplos:\n/check l[c]a-l[m]a\n/check [A]bc-[Z]bc")
                .await?;
            return Ok(());
        }
    };

    bot.send_message(chat_id, &format!("🔍 Escaneando patrón: {}...", pattern)).await?;

    let start = scan_pattern.start_char as u8;
    let end = scan_pattern.end_char as u8;
    let (start, end) = if start <= end { (start, end) } else { (end, start) };

    for c in start..=end {
        let current_char = c as char;
        let param = scan_pattern.template.replace("{}", &current_char.to_string());
        let url = format!("{}{}", BASE_URL, param);
        
        info!("Verificando: {}", url);
        
        match check_page(&url).await {
            Ok(Some(title)) => {
                if title != TEXTO_NO_DESEADO {
                    let message = format!("✅ Encontrado!\nURL: {}\nTítulo: {}", url, title);
                    bot.send_message(chat_id, &message).await?;
                }
            }
            Ok(None) => continue,
            Err(e) => {
                error!("Error: {}", e);
                continue;
            }
        }
        
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    bot.send_message(chat_id, "🚀 Escaneo completado!").await?;
    Ok(())
}

async fn check_page(url: &str) -> Result<Option<String>, reqwest::Error> {
    let res = reqwest::get(url).await?;
    let html = res.text().await?;
    
    let document = Html::parse_document(&html);
    let title_selector = Selector::parse("title").unwrap();
    
    Ok(document
        .select(&title_selector)
        .next()
        .map(|title| title.text().collect::<String>())
        .map(|t| t.trim().to_string()))
}

async fn start(bot: Bot, msg: Message) -> ResponseResult<()> {
    let help_text = "¡Bienvenido al Scraper Avanzado! 🕷️\n\n\
                   Comandos disponibles:\n\
                   /start - Muestra ayuda\n\
                   /check [patrón] - Inicia escaneo\n\n\
                   Ejemplos:\n\
                   • l[c]a-l[m]a → lca, lda,..., lma\n\
                   • [A]xx-[D]xx → Axx, Bxx, Cxx, Dxx";
    
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Iniciando bot...");

    let bot = Bot::from_env();

    let handler = Update::filter_message()
        .branch(dptree::entry().filter(|msg: Message| {
            msg.text().map(|text| text == "/start").unwrap_or(false)
        }).endpoint(start))
        .branch(dptree::entry()
            .filter(|msg: Message| {
                msg.text().map(|text| text.starts_with("/check ")).unwrap_or(false)
            })
            .endpoint(|bot: Bot, msg: Message| async move {
                let pattern = msg.text().unwrap().replace("/check ", "");
                check_links(bot, msg, pattern).await
            }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
