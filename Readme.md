# 🕷️ Telegram Scraper Bot 🤖

Un bot avanzado para escanear páginas web con patrones personalizados y notificar resultados via Telegram.

## 🚀 Características
- 🔍 Escaneo con patrones variables (ej: `lb[A]-lb[Z]`)
- ⏳ Delay configurable entre requests
- 🚫 Filtrado de contenido no deseado
- 📨 Notificaciones en tiempo real
- 🔄 Fácil despliegue en Termux

## 🎬 Integración con YTS
### Características de la integración YTS
- Monitoreo Automático : Verifica cada 3 minutos las nuevas películas en YTS.
- Notificaciones con Imágenes : Envía la portada de la película junto con la información.
- Enlaces Magnet : Genera automáticamente enlaces magnet para descarga directa.
- Múltiples Calidades : Muestra información sobre la calidad disponible de cada película.
- Sistema de Suscripción : Permite a los usuarios suscribirse/desuscribirse de las notificaciones.
### Comandos YTS
- /yts_init : Suscribe el chat actual a las notificaciones de nuevas películas y activa el monitor si no está en ejecución.
- /yts_stop : Cancela la suscripción del chat a las notificaciones de YTS.

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

## 🛠️ Instalación
```bash
pkg install git rust cargo
git clone https://github.com/tu_usuario/tu_repositorio.git
cd tu_repositorio
cargo run --release
```

## 🎮 Uso
```
/start - Muestra ayuda
/check [patrón] - Inicia escaneo

📌 Ejemplos:
/check lb[A]-lb[Z] → lbA, lbB,..., lbZ
/check [a]xx-[d]xx → axx, bxx, cxx, dxx
```

## 📦 Dependencias
- Rust 1.60+
- Cargo
- Termux API (opcional para portapapeles)


🔍 **Resumen del Proyecto: Telegram Scraper Bot**  

### 🧩 **Componentes Clave**  
1. **Patrones Dinámicos**:  
   - Sintaxis: `/check [A]xxx-[Z]xxx` → Genera URLs secuenciales (ej: `Axxx`, `Bxxx`, ..., `Zxxx`).  
   - Soporta mayúsculas/minúsculas y múltiples variables por patrón.  

2. **Filtro Inteligente**:  
   - Ignora contenido genérico (ej: títulos predeterminados).  
   - Notifica solo hallazgos relevantes.  

3. **Configuración Flexible**:  
   - Delay ajustable entre requests (2s por defecto).  
   - Token de bot gestionado por variables de entorno.  

### 🛠️ **Tecnologías**  
- **Rust**: Rendimiento y seguridad nativa.  
- **Teloxide**: Framework para bots de Telegram.  
- **Scraper**: Extracción eficiente de datos HTML.  
- **Termux**: Ejecución en dispositivos Android.  

### ⚡ **Cómo Funciona**  
1. **Parseo de Comando**:  
   - El usuario envía `/check lb[A]-lb[Z]`.  
   - El bot identifica el rango `A-Z` y plantilla `lb{}`.  

2. **Generación de URLs**:  
   - Crea secuencia: `lbA`, `lbB`, ..., `lbZ`.  

3. **Scraping y Filtrado**:  
   - Obtiene títulos de cada URL.  
   - Compara contra lista negra de contenido no deseado.  

4. **Notificaciones**:  
   - Envia mensajes solo con resultados válidos.  

### 📈 **Ventajas**  
- **Portabilidad**: Funciona en Android via Termux.  
- **Eficiencia**: Bajo consumo de recursos.  
- **Extensible**: Fácil adaptación a nuevos sitios web.  

### 🚀 **Casos de Uso**  
- Detección de nuevos episodios en plataformas de streaming.  
- Monitoreo de disponibilidad de productos en e-commerce.  
- Rastreo de actualizaciones en sitios de noticias.  

### 📌 **Requisitos**  
- Token de bot de Telegram (obtenido via @BotFather).  
- Rust 1.60+ y Cargo (gestor de paquetes).  
- Conexión a internet estable.  

**¡Un proyecto perfecto para entusiastas de la automatización y scraping!** 🚀
