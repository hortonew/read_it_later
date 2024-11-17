// Maintain a list of tags for synchronization
let tagsList = [];

document.addEventListener("DOMContentLoaded", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    const tabUrl = tab.url; // Get the active tab's URL
    const statusElement = document.getElementById("status");

    const removeButton = document.getElementById("removeUrlButton");
    const tagsInput = document.getElementById("tagsInput");

    // Disable buttons initially by removing 'enabled' class
    removeButton.classList.remove("enabled");
    tagsInput.classList.remove("enabled");

    // Attempt to send URL
    try {
        chrome.runtime.sendMessage(
            { action: "sendUrl", url: tabUrl },
            response => {
                if (response.status === "success") {
                    statusElement.textContent = "URL sent successfully!";
                    // Enable buttons
                    removeButton.classList.add("enabled");
                    tagsInput.classList.add("enabled");
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to send URL:", error);
        statusElement.textContent = `Error: ${error.message}`;
    }
});


// Remove URL handler
document.getElementById("removeUrlButton").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

    try {
        chrome.runtime.sendMessage(
            { action: "removeUrl", url: tab.url },
            response => {
                const statusElement = document.getElementById("status");
                if (response.status === "success") {
                    statusElement.textContent = "URL removed successfully!";
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to remove URL:", error);
    }
});

// Add tag on Enter or comma
document.getElementById("tagsInput").addEventListener("keydown", async function (event) {
    if (event.key === "," || event.key === "Enter") {
        event.preventDefault(); // Prevent adding the comma in the input field

        // Get the value of the input and remove whitespace and commas
        const input = event.target.value.trim().replace(/,$/, "");

        if (input && input !== "✖") {
            addTag(input);
            tagsList.push(input); // Add the tag to the list
            event.target.value = ""; // Clear the input field
            syncTags(); // Sync the updated tags list
        }
    }
});

// Function to add a tag to the UI
function addTag(text) {
    const tagContainer = document.getElementById("tagContainer");

    // Check for duplicate tags
    if (tagsList.includes(text)) {
        const statusElement = document.getElementById("status");
        statusElement.textContent = "Tag already exists!";
        return;
    }

    // Create a new tag element
    const tag = document.createElement("div");
    tag.className = "tag";

    // Add the tag text
    const span = document.createElement("span");
    span.textContent = text;
    tag.appendChild(span);

    // Add a remove button
    const close = document.createElement("span");
    close.textContent = "✖";
    close.className = "close";
    close.addEventListener("click", function () {
        tagContainer.removeChild(tag); // Remove tag on click
        removeTag(text); // Remove tag from the list
        syncTags(); // Sync the updated tags list
    });

    tag.appendChild(close);

    // Insert the tag before the input field
    const inputField = document.getElementById("tagsInput");
    tagContainer.insertBefore(tag, inputField);
}

// Function to remove a tag from the list
function removeTag(text) {
    const index = tagsList.indexOf(text);
    if (index > -1) {
        tagsList.splice(index, 1);
    }
}

// Function to sync tags with the server
async function syncTags() {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

    try {
        chrome.runtime.sendMessage(
            { action: "sendTags", url: tab.url, tags: tagsList.join(",") },
            response => {
                const statusElement = document.getElementById("status");
                if (response.status === "success") {
                    statusElement.textContent = "Tags synchronized successfully!";
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to sync tags:", error);
    }
}
