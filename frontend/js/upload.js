async function uploadFile(file) {
    const sigRes = await fetch("http://127.0.0.1:8080/cloudinary/signature");

    const sig = await sigRes.json();

    const formData = new FormData();

    formData.append("file", file);
    formData.append("api_key", sig.api_key);
    formData.append("timestamp", sig.timestamp);
    formData.append("signature", sig.signature);

    const uploadRes = await fetch(
        `https://api.cloudinary.com/v1_1/${sig.cloud_name}/auto/upload`,
        {
            method: "POST",
            body: formData,
        }
    );

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