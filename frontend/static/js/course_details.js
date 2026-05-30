const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
function goToModuleContent(moduleId) {
    window.location.href = "/module-content-page/" + moduleId;
}

let isInstructor = false;

async function loadSession() {
    const res = await axios.get("/debug-session");
    const session = res.data;

    const roles = session.role_names || [];

    isInstructor =
        roles.includes("Instructor") ||
        roles.includes("LMS Admin");

    if (isInstructor) {
        document.getElementById("instructor-controls").style.display = "flex";
    }
}

async function loadModules() {
    try {
        const response = await axios.get("/api/modules/" + courseId);
        const modules = response.data;

        const moduleList = document.getElementById("module-list");
        moduleList.innerHTML = "";
        if (modules.length === 0) {
            moduleList.innerHTML = "<p>No modules available.</p>";
            return;
        }
        modules.forEach((module, index) => {
            let instructorButtons = "";

            if (isInstructor) {
                instructorButtons = `
                    <div class="module-actions">
                        <button class="edit-btn" onclick="editModule(event, ${module.module_id})">Edit</button>
                        <button class="delete-btn" onclick="deleteModule(event, ${module.module_id})">Delete</button>
                    </div>
                `;
            }
            moduleList.innerHTML += `
                <div class="module-row" onclick="goToModuleContent(${module.module_id})">
                    <div class="module-info">
                        <div class="module-title">${module.title}</div>
                    </div>

                    ${instructorButtons}

                    <span class="module-arrow">›</span>
                </div>
            `;
        }
    )

    } catch (error) {
        console.error("Failed to load modules:", error);
    }
}
async function loadAssignments() {
    try {
        const response = await axios.get("/api/assignments/" + courseId);
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
        console.log("courseId =", courseId);

        const response = await axios.get("/api/courses/" + courseId);
        console.log("course response =", response.data);

        const course = response.data;

        document.getElementById("course-title").textContent = course.name;

        document.getElementById("course-hero").style.backgroundImage =
            `url('${course.background_image_url}')`;

    } catch (error) {
        console.error("Failed to load course title:", error);
    }
}
function editCourse(courseId) {
    event.stopPropagation();
    window.location.href = `/api/courses/${courseId}/edit`;
}

async function deleteCourse(courseId) {
    event.stopPropagation();

    if (!confirm("Delete this course?")) return;

    await axios.delete(`/api/courses/${courseId}`);
    loadModules();
}

document.getElementById("student-view-btn").onclick = () => {
    document.getElementById("instructor-controls").style.display = "none";
    isInstructor = false;
    loadModules();
};

function editModule(event, moduleId) {
    event.stopPropagation();
    window.location.href = `/module/${moduleId}/edit`;
}

async function deleteModule(event, moduleId) {
    event.stopPropagation();

    if (!confirm("Delete this module?")) return;

    await axios.delete(`/api/modules/${moduleId}`);
    loadModules();
}

document.getElementById("add-module-btn").onclick = () => {
    document.getElementById("add-module-modal").style.display = "flex";
};

document.getElementById("close-module-modal-btn").onclick = () => {
    document.getElementById("add-module-modal").style.display = "none";
};

document.getElementById("save-module-btn").onclick = async () => {
    const title = document.getElementById("module-title-input").value.trim();

    if (!title) {
        alert("Please enter a module title");
        return;
    }

    await axios.post("/api/modules", {
        course_id: Number(courseId),
        title: title,
        position: 999
    });

    document.getElementById("module-title-input").value = "";
    document.getElementById("add-module-modal").style.display = "none";

    loadModules();
};


async function init() {
    await loadSession();
    await loadCourseTitle();
    await loadModules();
    await loadAssignments();
}

init();
