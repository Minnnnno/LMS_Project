async function loadModules(courseId) {
    try {
        const response = await axios.get(
            "/module/" + courseId
        );

        const modules = response.data;

        const moduleGrid =
            document.getElementById("module-grid");

        moduleGrid.innerHTML = "";

        modules.forEach(module => {

            moduleGrid.innerHTML += `
                <div class="card module-card">
                    <div class="module-icon">�</div>

                    <h3>${course.name}</h3>

                    <p>
                        ${course.description || "No description"}
                    </p>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load courses:", error);
    }
}

loadCourses();