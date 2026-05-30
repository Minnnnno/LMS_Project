function goToCourse(courseId) {
    window.location.href = "/course-page/" + courseId;
}

async function loadCourses() {
    try {
        const response = await axios.get(
            "/api/courses"
        );

        const courses = response.data;

        const courseGrid =
            document.getElementById("course-grid");

        courseGrid.innerHTML = "";

        courses.forEach(course => {

            courseGrid.innerHTML += `
                <div class="modern-course-card"
                    onclick="goToCourse(${course.course_id})">
                    <div class="course-image"
                        style="background-image: url('${course.background_image_url}')">
                    </div>
                    <div class="course-content">

                        <h3 class="course-title">
                            ${course.name}
                        </h3>

                        <p class="course-description">
                            ${course.description || "No description"}
                        </p>

                    </div>
                </div>
            `;
        });

    } catch (error) {
        console.error("Failed to load courses:", error);
    }
}

loadCourses();