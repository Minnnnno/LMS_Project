const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
let currentCourse = null;
let actionMessageTimer = null;
let isInstructor = false;
let currentEditingModuleId = null;
let currentModules = [];

function goToModuleContent(moduleId) {
    window.location.href = "/module-content/" + moduleId;
}

function getCoursePriceCents(course) {
    if (Number.isFinite(Number(course.price))) {
        const decimalPriceCents = Math.round(Number(course.price) * 100);

        if (decimalPriceCents > 0) {
            return decimalPriceCents;
        }
    }

    if (Number.isFinite(Number(course.price_cents))) {
        return Number(course.price_cents);
    }

    return null;
}

function isPaidCourse(course) {
    const priceCents = getCoursePriceCents(course);
    return Boolean(course.is_paid) || (priceCents !== null && priceCents > 0);
}

function formatCoursePrice(course) {
    if (!isPaidCourse(course)) {
        return "Free course";
    }

    const priceCents = getCoursePriceCents(course);
    const currency = course.currency || "SGD";

    if (priceCents === null) {
        return "Price unavailable";
    }

    return new Intl.NumberFormat("en-SG", {
        style: "currency",
        currency,
    }).format(priceCents / 100);
}

function showActionMessage(message, type = "info") {
    const messageElement = document.getElementById("course-action-message");

    if (!messageElement) {
        return;
    }

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

    if (!button) {
        return;
    }

    button.disabled = disabled;
    button.innerHTML = content;
}

function resetCourseActionButton() {
    if (!currentCourse) {
        return;
    }

    if (isPaidCourse(currentCourse)) {
        setActionButton('<i class="bi bi-credit-card" aria-hidden="true"></i><span>Buy Course</span>');
    } else {
        setActionButton('<i class="bi bi-check2-circle" aria-hidden="true"></i><span>Enroll Now</span>');
    }
}

function configureCourseAction(course) {
    const price = document.getElementById("course-price");
    const params = new URLSearchParams(window.location.search);

    if (price) {
        price.textContent = formatCoursePrice(course);
    }

    resetCourseActionButton();

    if (params.get("payment") === "cancelled") {
        showActionMessage("Payment was cancelled. You can try again whenever you are ready.", "warning");
    }
}

function refreshCourseDisplay() {
    if (!currentCourse) {
        return;
    }

    document.getElementById("course-title").textContent = currentCourse.name || "Untitled course";

    if (currentCourse.background_image_url) {
        document.getElementById("course-hero").style.backgroundImage =
            `url('${currentCourse.background_image_url}')`;
    }

    configureCourseAction(currentCourse);
}

async function handleCourseAction() {
    if (!currentCourse) {
        return;
    }

    setActionButton(
        isPaidCourse(currentCourse)
            ? '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Opening checkout...</span>'
            : '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Enrolling...</span>',
        true
    );
    showActionMessage("");

    try {
        if (isPaidCourse(currentCourse)) {
            const response = await axios.post(`/api/courses/${courseId}/checkout`);
            window.location.href = response.data.checkout_url;
            return;
        }

        await axios.post(`/api/courses/${courseId}/enroll`);
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
        const modules = response.data.sort((first, second) => first.position - second.position);
        currentModules = modules;
        const moduleList = document.getElementById("module-list");

        moduleList.innerHTML = "";

        if (modules.length === 0) {
            moduleList.innerHTML = "<p>No modules available.</p>";
            return;
        }

        modules.forEach((module) => {
            const instructorButtons = isInstructor
                ? `
                    <div class="module-actions">
                        <button class="module-action-btn edit-btn" onclick="editModule(event, ${module.module_id})">Edit</button>
                        <button class="module-action-btn delete-btn" onclick="deleteModule(event, ${module.module_id})">Delete</button>
                    </div>
                `
                : "";

            moduleList.innerHTML += `
                <div class="module-row" onclick="goToModuleContent(${module.module_id})">
                    <div class="module-info">
                        <div class="module-title">${module.title}</div>
                    </div>
                    ${instructorButtons}
                    <span class="module-arrow">&rsaquo;</span>
                </div>
            `;
        });
    } catch (error) {
        console.error("Failed to load modules:", error);
    }
}

async function loadAssignments() {
    try {
        const response = await axios.get("/api/assignment/" + courseId);
        const assignments = response.data;
        const assignmentList = document.getElementById("assignment-list");

        assignmentList.innerHTML = "";

        if (!assignments.length) {
            assignmentList.innerHTML = "<p>No tasks due.</p>";
            return;
        }

        assignments.forEach((assignment) => {
            assignmentList.innerHTML += `
                <div class="assignment-row">
                    <div>
                        <div class="assignment-title">${assignment.title}</div>
                        <div class="assignment-subtitle">Due: ${assignment.due_date}</div>
                    </div>
                </div>
            `;
        });
    } catch (error) {
        const assignmentList = document.getElementById("assignment-list");
        assignmentList.innerHTML = "<p>No tasks due.</p>";
        console.error("Failed to load assignments:", error);
    }
}

async function loadCourseTitle() {
    try {
        const response = await axios.get("/api/course/" + courseId);
        currentCourse = response.data;

        refreshCourseDisplay();
    } catch (error) {
        console.error("Failed to load course title:", error);
        showActionMessage("Failed to load course details.", "error");
    }
}

async function loadManageAccess() {
    try {
        const response = await axios.get(`/api/courses/${courseId}/manage-access`);
        isInstructor = Boolean(response.data.can_manage);

        const controls = document.getElementById("instructor-controls");
        if (controls) {
            controls.style.display = isInstructor ? "flex" : "none";
        }

        const actionStrip = document.querySelector(".course-action-strip");
        if (actionStrip) {
            actionStrip.style.display = isInstructor ? "none" : "grid";
        }
    } catch (error) {
        isInstructor = false;
        const actionStrip = document.querySelector(".course-action-strip");
        if (actionStrip) {
            actionStrip.style.display = "grid";
        }
        console.error("Failed to load course management access:", error);
    }
}

function editCourse(event, courseId) {
    event.stopPropagation();
    openCourseModal();
}

async function deleteCourse(event, courseId) {
    event.stopPropagation();

    if (!confirm("Delete this course?")) {
        return;
    }

    try {
        await axios.delete(`/api/courses/${courseId}`);
        window.location.href = "/courses";
    } catch (error) {
        const message = error.response?.data || "Failed to delete course.";
        showActionMessage(message, "error");
    }
}

function openCourseModal() {
    if (!currentCourse) {
        return;
    }

    document.getElementById("course-name-input").value = currentCourse.name || "";
    document.getElementById("course-name-input").placeholder = currentCourse.name || "Course name";
    document.getElementById("course-description-input").value = currentCourse.description || "";
    document.getElementById("course-description-input").placeholder = currentCourse.description || "Course description";
    document.getElementById("course-image-input").value = "";
    const priceCents = getCoursePriceCents(currentCourse);
    document.getElementById("course-price-input").value =
        priceCents === null ? "" : (priceCents / 100).toFixed(2);
    document.getElementById("course-price-input").placeholder =
        priceCents === null ? "0.00" : (priceCents / 100).toFixed(2);
    document.getElementById("course-currency-input").value = currentCourse.currency || "SGD";
    document.getElementById("course-currency-input").placeholder = currentCourse.currency || "SGD";
    document.getElementById("course-status-input").value = currentCourse.status || "draft";
    document.getElementById("course-paid-input").checked = Boolean(currentCourse.is_paid);
    document.getElementById("edit-course-modal").style.display = "flex";
}

function closeCourseModal() {
    document.getElementById("edit-course-modal").style.display = "none";
}

async function uploadCourseImage(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/courses");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data.secure_url;
}

async function saveCourse() {
    const name = document.getElementById("course-name-input").value.trim();
    const description = document.getElementById("course-description-input").value.trim();
    const backgroundImageFile = document.getElementById("course-image-input").files[0];
    const priceInputValue = document.getElementById("course-price-input").value.trim();
    const currency = document.getElementById("course-currency-input").value.trim() || "SGD";
    const status = document.getElementById("course-status-input").value;
    const isPaid = document.getElementById("course-paid-input").checked;

    if (!name) {
        alert("Please enter a course name");
        return;
    }

    try {
        const backgroundImageUrl = backgroundImageFile
            ? await uploadCourseImage(backgroundImageFile)
            : currentCourse.background_image_url;

        const payload = {
            name,
            description: description || null,
            background_image_url: backgroundImageUrl || null,
            currency,
            status,
            is_paid: isPaid,
        };

        if (priceInputValue !== "") {
            payload.price = Number(priceInputValue);
        }

        await axios.put(`/api/courses/${courseId}`, payload);

        closeCourseModal();
        await loadCourseTitle();
        showActionMessage("Course updated.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to update course.";
        showActionMessage(message, "error");
    }
}

function editModule(event, moduleId) {
    event.stopPropagation();
    const module = currentModules.find((item) => item.module_id === moduleId);

    if (!module) {
        return;
    }

    openModuleModal(module);
}

async function deleteModule(event, moduleId) {
    event.stopPropagation();

    if (!confirm("Delete this module?")) {
        return;
    }

    await axios.delete(`/api/module/${moduleId}`);
    loadModules();
}

function openModuleModal(module = null) {
    currentEditingModuleId = module?.module_id || null;
    document.getElementById("module-modal-title").textContent = module ? "Edit Module" : "Add Module";
    document.getElementById("module-title-input").value = module?.title || "";
    document.getElementById("module-title-input").placeholder = module?.title || "Module title, e.g. Week 1 Introduction";
    document.getElementById("module-position-input").value =
        module?.position || currentModules.length + 1;
    document.getElementById("module-position-input").placeholder =
        String(module?.position || currentModules.length + 1);
    document.getElementById("add-module-modal").style.display = "flex";
}

function closeModuleModal() {
    currentEditingModuleId = null;
    document.getElementById("module-title-input").value = "";
    document.getElementById("module-title-input").placeholder = "Module title, e.g. Week 1 Introduction";
    document.getElementById("module-position-input").value = "";
    document.getElementById("module-position-input").placeholder = "1";
    document.getElementById("add-module-modal").style.display = "none";
}

async function saveModule() {
    const title = document.getElementById("module-title-input").value.trim();
    const position = Number(document.getElementById("module-position-input").value || 0);

    if (!title) {
        alert("Please enter a module title");
        return;
    }

    if (!Number.isInteger(position) || position < 1) {
        alert("Please enter a display order of 1 or higher");
        return;
    }

    if (currentEditingModuleId) {
        const currentModule = currentModules.find((module) => module.module_id === currentEditingModuleId);

        if (currentModule && currentModule.position !== position) {
            const confirmed = confirm(`Do you want to move ${title} to order ${position}?`);

            if (!confirmed) {
                return;
            }
        }

        await axios.put(`/api/modules/${currentEditingModuleId}`, {
            title,
            position,
        });
    } else {
        await axios.post("/api/modules", {
            course_id: Number(courseId),
            title,
            position,
        });
    }

    closeModuleModal();
    loadModules();
}

function bindInstructorControls() {
    document.getElementById("edit-course-btn")?.addEventListener("click", (event) => {
        editCourse(event, courseId);
    });

    document.getElementById("delete-course-btn")?.addEventListener("click", (event) => {
        deleteCourse(event, courseId);
    });

    document.getElementById("save-course-btn")?.addEventListener("click", saveCourse);
    document.getElementById("close-course-modal-btn")?.addEventListener("click", closeCourseModal);

    document.getElementById("student-view-btn")?.addEventListener("click", () => {
        document.getElementById("instructor-controls").style.display = "none";
        isInstructor = false;
        loadModules();
    });

    document.getElementById("add-module-btn")?.addEventListener("click", () => {
        openModuleModal();
    });

    document.getElementById("close-module-modal-btn")?.addEventListener("click", () => {
        closeModuleModal();
    });

    document.getElementById("save-module-btn")?.addEventListener("click", saveModule);
}

async function init() {
    document.getElementById("course-action-button")
        ?.addEventListener("click", handleCourseAction);

    bindInstructorControls();

    await loadCourseTitle();
    await loadManageAccess();
    await loadModules();
    await loadAssignments();
}

init();
