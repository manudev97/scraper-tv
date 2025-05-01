# 🕷️ Telegram Scraper Bot 🤖

Un bot avanzado para escanear páginas web con patrones personalizados y notificar resultados via Telegram.

## 🚀 Características
- 🔍 Escaneo con patrones variables (ej: `lb[A]-lb[Z]`)
- ⏳ Delay configurable entre requests
- 🚫 Filtrado de contenido no deseado
- 📨 Notificaciones en tiempo real
- 🔄 Fácil despliegue en Termux

## ⚙️ Configuración

### 1. Obtener Token de BotFather
1. Abre Telegram y busca `@BotFather`
2. Crea un nuevo bot con `/newbot`
3. Copia el token generado

### 2. Configurar Entorno (Termux)
```bash
# Método temporal (solo esta sesión)
export TELOXIDE_TOKEN="TU_TOKEN_AQUI"

# Método permanente
echo 'export TELOXIDE_TOKEN="TU_TOKEN_AQUI"' >> ~/.bashrc
source ~/.bashrc

# O usar .env
echo "TELOXIDE_TOKEN=TU_TOKEN_AQUI" > .env
