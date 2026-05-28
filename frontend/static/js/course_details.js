const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
function goToModuleContent(moduleId) {
    window.location.href = "/module-content-page/" + moduleId;
}

async function loadModules() {
    try {
        const response = await axios.get("/module/" + courseId);
        const modules = response.data;

        const moduleList = document.getElementById("module-list");
        moduleList.innerHTML = "";

        modules.forEach(module => {
            moduleList.innerHTML += `
                <div class="module-row" onclick="goToModuleContent(${module.module_id})">
                    <span>${module.title}</span>
                    <span class="module-arrow">›</span>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load modules:", error);
    }
}


loadModules();