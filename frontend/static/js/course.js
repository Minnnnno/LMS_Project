function goToCourse(courseId) {
    window.location.href = "/course/" + courseId;
}

async function loadCourses() {
    try {
        const response = await axios.get(
            "/allcourses"
        );

        const courses = response.data;

        const courseGrid =
            document.getElementById("course-grid");

        courseGrid.innerHTML = "";

        courses.forEach(course => {

            courseGrid.innerHTML += `
                <div class="card course-card" onclick="goToCourse(${course.course_id})">
                    <div class="course-icon">📘</div>

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