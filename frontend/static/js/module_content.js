const pathParts = window.location.pathname.split("/");
const moduleId = pathParts[2];

async function loadModuleContent() {
    try {
        const response = await axios.get("/module-content/" + moduleId);
        const contents = response.data;

        const contentList = document.getElementById("content-list");
        contentList.innerHTML = "";

        contents.forEach(content => {
            contentList.innerHTML += `
                <div class="module-row" onclick="openContent('${content.content_url}')">
                    <span>${content.title}</span>
                    <span class="module-arrow">✓</span>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load module content:", error);
    }
}

function openContent(url) {
    window.open(url, "_blank");
}

loadModuleContent();