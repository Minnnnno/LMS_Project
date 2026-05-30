const pathParts = window.location.pathname.split("/");
const moduleId = pathParts[2];

async function loadModuleContent() {
    try {
        const response = await axios.get("/api/module-content/" + moduleId);
        const contents = Array.isArray(response.data)
            ? response.data
            : [response.data];
        const contentList = document.getElementById("content-list");
        contentList.innerHTML = "";

        contents.forEach(content => {
            contentList.innerHTML += `
                <div class="module-row" onclick="openPdf('${content.content_url}')">
                    <span>${content.title}</span>
                    <span class="module-arrow">›</span>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load module content:", error);
    }
}

function openPdf(pdfUrl) {
    const encodedUrl = encodeURIComponent(pdfUrl);
    window.location.href = `/pdf-viewer-page?url=${encodedUrl}`;
}

loadModuleContent();