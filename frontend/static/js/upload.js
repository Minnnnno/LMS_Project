async function uploadFile(file) {
    const formData = new FormData();

    formData.append("file", file);
    formData.append("folder", "lms/uploads");

    const uploadRes = await fetch("/api/cloudinary/upload", {
        method: "POST",
        body: formData,
    });

    const data = await uploadRes.json();

    console.log(data);

    return data.secure_url;
}

async function testUpload() {

    const file = document.getElementById("fileInput").files[0];

    if (!file) {
        alert("Please choose a file");
        return;
    }

    const url = await uploadFile(file);

    console.log(url);

    alert("Upload success!");
}
