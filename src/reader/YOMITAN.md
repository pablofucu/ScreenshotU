# Yomitan en la página de lectura

La página de lectura se abre en Edge/Chrome (modo app) con un **perfil propio de la app**, así que Yomitan hay que instalarlo una sola vez en ese perfil.

1. Con la página de lectura abierta, abre el menú `⋮` de la ventana (o `Alt+F`) → *Extensiones*. Si no hay menú en modo app, abre el mismo navegador con el perfil de la app y entra en `edge://extensions` (o `chrome://extensions`).
2. Instala Yomitan: Edge Add-ons / Chrome Web Store → «Yomitan». Si es Chrome o Edge sin tienda, descarga el `.zip` de <https://github.com/yomidevs/yomitan/releases> y cárgalo con *Modo desarrollador → Cargar descomprimida*.
3. Fija la extensión en la barra y permite que funcione en `http://127.0.0.1` (Detalles de la extensión → *Permitir acceso a la URL del archivo* no es necesario; basta con el acceso al sitio).
4. Abre los ajustes de Yomitan → **Diccionarios** → *Importar* y elige JMdict (p. ej. `JMdict_english` o `jmdict-es` para español, y opcionalmente JMnedict, KANJIDIC y un diccionario de frecuencias). Descargas: <https://github.com/yomidevs/yomitan/blob/master/docs/dictionaries.md>.
5. Ajustes → **Apariencia** → *Popup CSS* (en «Opciones avanzadas» si no se ve): pega el contenido de `yomitan-popup.css`. También lo da la app con `novel_yomitan_css()`.
6. Escaneo: por defecto, mantén `Shift` y pasa el ratón sobre el texto japonés (se puede cambiar en Ajustes → *Escaneo*). El texto de la página es seleccionable y el furigana no interfiere con el escaneo.

El perfil queda guardado, por lo que los diccionarios y ajustes se conservan entre sesiones.

## Acepciones en la propia página

Debajo de la frase la página muestra las palabras principales con sus acepciones. Las saca la app (no Yomitan) de los
diccionarios en formato Yomitan que haya en `<config>/dicts` (botón *Diccionarios* de la página): copia ahí el mismo
`.zip` que importaste en Yomitan (JMdict, `jmdict-es`...) y pulsa *Recargar*. Los diccionarios de kanji o de frecuencias se ignoran.
