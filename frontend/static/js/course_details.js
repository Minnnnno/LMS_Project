const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
let currentCourse = null;
let actionMessageTimer = null;
let isInstructor = false;
let currentEditingModuleId = null;
let currentModules = [];
let currentEditingAssignmentId = null;
let currentAssignments = [];
let currentAssignmentBriefUrl = null;

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
        currentAssignments = assignments;
        const assignmentList = document.getElementById("assignment-list");

        assignmentList.innerHTML = "";

        if (!assignments.length) {
            assignmentList.innerHTML = isInstructor
                ? '<p class="assignment-empty">No assignments yet. Use Add Assignment to create one.</p>'
                : '<p class="assignment-empty">No tasks due.</p>';
            return;
        }

        assignments.forEach((assignment) => {
            const adminButtons = isInstructor
                ? `
                    <div class="module-actions">
                        <button class="module-action-btn edit-btn" onclick="editAssignment(event, ${assignment.assignment_id})">Edit</button>
                        <button class="module-action-btn delete-btn" onclick="deleteAssignment(event, ${assignment.assignment_id})">Delete</button>
                    </div>
                `
                : "";

            assignmentList.innerHTML += `
                <div class="assignment-row" onclick="openAssignmentDetails(${assignment.assignment_id})">
                    <div>
                        <div class="assignment-title">${assignment.title}</div>
                        <div class="assignment-subtitle">Due: ${formatAssignmentDate(assignment.due_date)}</div>
                    </div>
                    ${adminButtons}
                </div>
            `;
        });
    } catch (error) {
        currentAssignments = [];
        const assignmentList = document.getElementById("assignment-list");
        assignmentList.innerHTML = "<p>No tasks due.</p>";
        console.error("Failed to load assignments:", error);
    }
}

function openAssignmentDetails(assignmentId) {
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    document.getElementById("assignment-details-title").textContent = assignment.title || "Assignment Details";
    document.getElementById("assignment-details-description").textContent =
        assignment.description || "No description provided.";
    document.getElementById("assignment-details-due").textContent = formatAssignmentDate(assignment.due_date);
    document.getElementById("assignment-details-score").textContent = assignment.max_score ?? "Not set";
    document.getElementById("assignment-details-file-type").textContent =
        getFileTypeLabel(assignment.expected_file_type);
    document.getElementById("assignment-details-submission").textContent =
        getAssignmentSubmissionLabel(assignment);

    const briefWrap = document.getElementById("assignment-details-brief-wrap");
    const briefLink = document.getElementById("assignment-details-brief-link");
    if (assignment.assignment_brief_url) {
        briefLink.href = assignment.assignment_brief_url;
        briefWrap.style.display = "block";
    } else {
        briefLink.href = "#";
        briefWrap.style.display = "none";
    }

    const instructionsWrap = document.getElementById("assignment-details-instructions-wrap");
    const instructions = document.getElementById("assignment-details-instructions");
    if (assignment.submission_instructions) {
        instructions.textContent = assignment.submission_instructions;
        instructionsWrap.style.display = "block";
    } else {
        instructions.textContent = "";
        instructionsWrap.style.display = "none";
    }

    document.getElementById("assignment-details-modal").style.display = "flex";
}

function closeAssignmentDetails() {
    document.getElementById("assignment-details-modal").style.display = "none";
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

        setAssignmentCardAddVisible(isInstructor);

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
        setAssignmentCardAddVisible(false);
        console.error("Failed to load course management access:", error);
    }
}

function setAssignmentCardAddVisible(visible) {
    const assignmentCardAddButton = document.getElementById("assignment-card-add-btn");

    if (assignmentCardAddButton) {
        assignmentCardAddButton.style.display = visible ? "inline-flex" : "none";
    }
}

function populateDueTimeOptions() {
    const timeSelect = document.getElementById("assignment-due-time-input");

    if (!timeSelect || timeSelect.options.length) {
        return;
    }

    for (let hour = 0; hour < 24; hour += 1) {
        for (const minute of ["00", "30"]) {
            const value = `${String(hour).padStart(2, "0")}:${minute}`;
            timeSelect.innerHTML += `<option value="${value}">${value}</option>`;
        }
    }
}

function setAssignmentSaveState(isSaving, message = "") {
    const saveButton = document.getElementById("save-assignment-btn");
    const closeButton = document.getElementById("close-assignment-modal-btn");
    const status = document.getElementById("assignment-save-status");

    if (saveButton) {
        saveButton.disabled = isSaving;
        saveButton.textContent = isSaving ? "Saving..." : "Save";
    }

    if (closeButton) {
        closeButton.disabled = isSaving;
    }

    if (status) {
        status.textContent = message;
    }
}

function formatAssignmentDate(value) {
    if (!value) {
        return "No due date";
    }

    const date = new Date(value);

    if (Number.isNaN(date.getTime())) {
        return value;
    }

    return date.toLocaleString("en-SG", {
        dateStyle: "medium",
        timeStyle: "short",
    });
}

function toDatetimeLocalValue(value) {
    if (!value) {
        return "";
    }

    const date = new Date(value);

    if (Number.isNaN(date.getTime())) {
        return value.slice(0, 16);
    }

    const localDate = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
    return localDate.toISOString().slice(0, 16);
}

function getDateInputValue(value) {
    return toDatetimeLocalValue(value).slice(0, 10);
}

function getTimeInputValue(value) {
    const localValue = toDatetimeLocalValue(value);
    return localValue.length >= 16 ? localValue.slice(11, 16) : "00:00";
}

function toApiDateTime(value) {
    return value ? `${value}:00` : null;
}

function getAssignmentSubmissionLabel(assignment) {
    const methods = [];

    if (assignment.allow_text_submission ?? true) {
        methods.push("Text");
    }

    if (assignment.allow_file_submission ?? true) {
        methods.push("File");
    }

    return methods.length ? methods.join(" and ") : "No submission method set";
}

function getFileTypeLabel(value) {
    const labels = {
        pdf: "PDF",
        docx: "Word document (.docx)",
        pptx: "PowerPoint (.pptx)",
        xlsx: "Excel spreadsheet (.xlsx)",
        zip: "ZIP archive",
        image: "Image",
    };

    return labels[value] || "Any file type";
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

async function uploadAssignmentBrief(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/assignments");

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

function openAssignmentModal(assignment = null) {
    populateDueTimeOptions();
    setAssignmentSaveState(false, "");
    currentEditingAssignmentId = assignment?.assignment_id || null;
    currentAssignmentBriefUrl = assignment?.assignment_brief_url || null;
    document.getElementById("assignment-modal-title").textContent = assignment ? "Edit Assignment" : "Add Assignment";
    document.getElementById("assignment-title-input").value = assignment?.title || "";
    document.getElementById("assignment-description-input").value = assignment?.description || "";
    document.getElementById("assignment-due-date-input").value = getDateInputValue(assignment?.due_date);
    document.getElementById("assignment-due-time-input").value = getTimeInputValue(assignment?.due_date);
    document.getElementById("assignment-score-input").value = assignment?.max_score ?? "";
    document.getElementById("assignment-brief-file-input").value = "";
    const briefLink = document.getElementById("assignment-brief-current-link");
    if (briefLink) {
        briefLink.href = currentAssignmentBriefUrl || "#";
        briefLink.style.display = currentAssignmentBriefUrl ? "inline-flex" : "none";
    }
    document.getElementById("assignment-file-type-input").value = assignment?.expected_file_type || "";
    document.getElementById("assignment-text-input").checked = assignment?.allow_text_submission ?? true;
    document.getElementById("assignment-file-input").checked = assignment?.allow_file_submission ?? true;
    document.getElementById("assignment-file-size-input").value = assignment?.max_file_size_mb ?? "";
    document.getElementById("assignment-instructions-input").value = assignment?.submission_instructions || "";
    document.getElementById("assignment-modal").style.display = "flex";
}

function closeAssignmentModal() {
    currentEditingAssignmentId = null;
    currentAssignmentBriefUrl = null;
    document.getElementById("assignment-modal").style.display = "none";
}

function editAssignment(event, assignmentId) {
    event.stopPropagation();
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    openAssignmentModal(assignment);
}

async function deleteAssignment(event, assignmentId) {
    event.stopPropagation();

    if (!confirm("Delete this assignment?")) {
        return;
    }

    try {
        await axios.delete(`/api/assignment/${assignmentId}`);
        await loadAssignments();
        showActionMessage("Assignment deleted.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to delete assignment.";
        showActionMessage(message, "error");
    }
}

async function saveAssignment() {
    const title = document.getElementById("assignment-title-input").value.trim();
    const description = document.getElementById("assignment-description-input").value.trim();
    const dueDate = document.getElementById("assignment-due-date-input").value;
    const dueTime = document.getElementById("assignment-due-time-input").value;
    const maxScore = document.getElementById("assignment-score-input").value.trim();
    const briefFile = document.getElementById("assignment-brief-file-input").files[0];
    const expectedFileType = document.getElementById("assignment-file-type-input").value;
    const maxFileSize = document.getElementById("assignment-file-size-input").value.trim();
    const submissionInstructions = document.getElementById("assignment-instructions-input").value.trim();

    if (!title) {
        alert("Please enter an assignment title");
        return;
    }

    if (!description) {
        alert("Please enter an assignment description");
        return;
    }

    if (!dueDate) {
        alert("Please choose a due date");
        return;
    }

    if (maxScore === "" || Number(maxScore) < 0) {
        alert("Please enter a max score of 0 or higher");
        return;
    }

    try {
        setAssignmentSaveState(true, briefFile ? "Uploading assignment brief..." : "Saving assignment...");

        let briefUrl = currentAssignmentBriefUrl;

        if (briefFile) {
            briefUrl = await uploadAssignmentBrief(briefFile);
            setAssignmentSaveState(true, "Brief uploaded. Saving assignment...");
        }

        const payload = {
            course_id: Number(courseId),
            title,
            description,
            due_date: toApiDateTime(`${dueDate}T${dueTime}`),
            max_score: Number(maxScore),
            assignment_brief_url: briefUrl || null,
            expected_file_type: expectedFileType || null,
            allow_text_submission: document.getElementById("assignment-text-input").checked,
            allow_file_submission: document.getElementById("assignment-file-input").checked,
            max_file_size_mb: maxFileSize ? Number(maxFileSize) : null,
            submission_instructions: submissionInstructions || null,
        };

        if (currentEditingAssignmentId) {
            await axios.put(`/api/assignment/${currentEditingAssignmentId}`, payload);
        } else {
            await axios.post("/api/assignment", payload);
        }

        closeAssignmentModal();
        await loadAssignments();
        showActionMessage("Assignment saved.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to save assignment.";
        setAssignmentSaveState(false, "");
        showActionMessage(message, "error");
    }
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
    await loadModules();
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
        setAssignmentCardAddVisible(false);
        loadModules();
        loadAssignments();
    });

    document.getElementById("add-module-btn")?.addEventListener("click", () => {
        openModuleModal();
    });

    document.getElementById("add-assignment-btn")?.addEventListener("click", () => {
        openAssignmentModal();
    });

    document.getElementById("assignment-card-add-btn")?.addEventListener("click", () => {
        openAssignmentModal();
    });

    document.getElementById("close-module-modal-btn")?.addEventListener("click", () => {
        closeModuleModal();
    });

    document.getElementById("save-module-btn")?.addEventListener("click", saveModule);
    document.getElementById("save-assignment-btn")?.addEventListener("click", saveAssignment);
    document.getElementById("close-assignment-modal-btn")?.addEventListener("click", closeAssignmentModal);
    document.getElementById("close-assignment-details-btn")?.addEventListener("click", closeAssignmentDetails);
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
