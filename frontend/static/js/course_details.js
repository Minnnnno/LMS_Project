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

        if (modules.length === 0) {
            moduleList.innerHTML = "<p>No modules available.</p>";
            return;
        }

        modules.forEach((module, index) => {
            moduleList.innerHTML += `
                <div class="module-row" onclick="goToModuleContent(${module.module_id})">
                    <div>
                        <div class="module-title">${module.title}</div>
                    </div>
                    <span class="module-arrow">›</span>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load modules:", error);
    }
}

async function loadAssignments() {
    try {
        const response = await axios.get("/assignment/" + courseId);
        const assignments = response.data;

        const assignmentList = document.getElementById("assignment-list");
        assignmentList.innerHTML = "";

        assignments.forEach(assignment => {
            assignmentList.innerHTML += `
                <div class="assignment-row">
                    <div>
                        <div class="assignment-title">${assignment.title}</div>
                        <div class="assignment-subtitle">
                            Due: ${assignment.due_date}
                        </div>
                    </div>
                </div>
            `;
        });

    } catch (error) {
        const assignmentList = document.getElementById("assignment-list");
        assignmentList.innerHTML = "<p>No assignments due.</p>";
        console.error("Failed to load assignments:", error);
    }
}

async function loadCourseTitle() {
    try {
        const response = await axios.get("/course/" + courseId);

        const course = response.data;

        document.getElementById("course-title")
            .textContent = course.name;

        document.getElementById("course-hero").style.backgroundImage =
    `url('${course.background_image_url}')`;

    } catch (error) {
        console.error("Failed to load course title:", error);
    }
}


loadCourseTitle();
loadModules();
loadAssignments();