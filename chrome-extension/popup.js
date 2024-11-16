document.getElementById("sendUrlButton").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

    chrome.runtime.sendMessage(
        { action: "sendUrl", url: tab.url },
        response => {
            const statusElement = document.getElementById("status");
            if (response.status === "success") {
                statusElement.textContent = "URL sent successfully!";
            } else {
                statusElement.textContent = `Error: ${response.error}`;
            }
        }
    );
});
