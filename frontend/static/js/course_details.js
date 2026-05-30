const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
let currentCourse = null;
let actionMessageTimer = null;

function goToModuleContent(moduleId) {
    window.location.href = "/module-content-page/" + moduleId;
}

function formatCoursePrice(course) {
    if (!course.is_paid) {
        return "Free course";
    }

    const priceCents = course.price_cents || 0;
    const currency = course.currency || "SGD";

    return new Intl.NumberFormat("en-SG", {
        style: "currency",
        currency,
    }).format(priceCents / 100);
}

function showActionMessage(message, type = "info") {
    const messageElement = document.getElementById("course-action-message");

    if (actionMessageTimer) {
        clearTimeout(actionMessageTimer);
    }

    messageElement.textContent = message;
    messageElement.className = message
        ? `course-action-message ${type} visible`
        : "course-action-message";

    if (message) {
        actionMessageTimer = setTimeout(() => {
            messageElement.classList.remove("visible");
        }, 4500);
    }
}

function setActionButton(content, disabled = false) {
    const button = document.getElementById("course-action-button");
    button.disabled = disabled;
    button.innerHTML = content;
}

function resetCourseActionButton() {
    if (!currentCourse) {
        return;
    }

    if (currentCourse.is_paid) {
        setActionButton('<i class="bi bi-credit-card" aria-hidden="true"></i><span>Buy Course</span>');
    } else {
        setActionButton('<i class="bi bi-check2-circle" aria-hidden="true"></i><span>Enroll Now</span>');
    }
}

function configureCourseAction(course) {
    const price = document.getElementById("course-price");
    const params = new URLSearchParams(window.location.search);

    price.textContent = formatCoursePrice(course);
    resetCourseActionButton();

    if (params.get("payment") === "cancelled") {
        showActionMessage("Payment was cancelled. You can try again whenever you are ready.", "warning");
    }
}

async function handleCourseAction() {
    if (!currentCourse) {
        return;
    }

    setActionButton(
        currentCourse.is_paid
            ? '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Opening checkout...</span>'
            : '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Enrolling...</span>',
        true
    );
    showActionMessage("");

    try {
        if (currentCourse.is_paid) {
            const response = await axios.post(`/courses/${courseId}/checkout`);
            window.location.href = response.data.checkout_url;
            return;
        }

        await axios.post(`/courses/${courseId}/enroll`);
        setActionButton('<i class="bi bi-check2" aria-hidden="true"></i><span>Enrolled</span>', true);
        showActionMessage("You are enrolled in this course.", "success");
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Something went wrong. Please try again.";
        showActionMessage(message, "error");
        resetCourseActionButton();
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

        currentCourse = response.data;

<<<<<<< HEAD
        document.getElementById("course-title").textContent = course.name;

        document.getElementById("course-hero").style.backgroundImage =
            `url('${course.background_image_url}')`;
=======
        document.getElementById("course-title")
            .textContent = currentCourse.name;

        document.getElementById("course-hero").style.backgroundImage =
    `url('${currentCourse.background_image_url}')`;

        configureCourseAction(currentCourse);
>>>>>>> 88a53759a33d7c4553c643c15d0557c03ebdb1e9

    } catch (error) {
        console.error("Failed to load course title:", error);
        showActionMessage("Failed to load course details.", "error");
    }
}
function editCourse(courseId) {
    event.stopPropagation();
    window.location.href = `/api/courses/${courseId}/edit`;
}

<<<<<<< HEAD
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
=======
document.getElementById("course-action-button")
    ?.addEventListener("click", handleCourseAction);

loadCourseTitle();
loadModules();
loadAssignments();
>>>>>>> 88a53759a33d7c4553c643c15d0557c03ebdb1e9
