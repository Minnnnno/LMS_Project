const params = new URLSearchParams(window.location.search);
const pdfUrl = params.get("url");

document.getElementById("pdf-viewer").src = pdfUrl;