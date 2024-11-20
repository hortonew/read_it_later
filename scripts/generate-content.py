import random
import string

import requests

# Server configuration
BASE_URL = "http://localhost:8080"

# Sample tags dictionary
TAGS = [
    "python",
    "javascript",
    "chrome-extension",
    "automation",
    "api",
    "testing",
    "random",
    "example",
    "development",
    "snippet",
]

# List of top domain names
TOP_DOMAINS = [
    "google.com",
    "youtube.com",
    "facebook.com",
    "baidu.com",
    "wikipedia.org",
    "reddit.com",
    "yahoo.com",
    "qq.com",
    "taobao.com",
    "amazon.com",
    "tmall.com",
    "twitter.com",
    "instagram.com",
    "vk.com",
    "live.com",
    "sohu.com",
    "sina.com.cn",
    "jd.com",
    "weibo.com",
    "360.cn",
    "linkedin.com",
    "netflix.com",
]


# Helper functions
def random_string(length=10):
    """Generate a random alphanumeric string."""
    return "".join(random.choices(string.ascii_letters + string.digits, k=length))


def random_sentence():
    """Generate a random sentence."""
    subjects = ["The cat", "A dog", "The bird", "A fish", "The monkey"]
    verbs = ["jumps", "runs", "flies", "swims", "climbs"]
    objects = [
        "over the fence",
        "in the park",
        "through the sky",
        "in the water",
        "up the tree",
    ]
    return f"{random.choice(subjects)} {random.choice(verbs)} {random.choice(objects)}."


def random_quote():
    """Generate a random quote."""
    quotes = [
        "To be or not to be, that is the question.",
        "The only thing we have to fear is fear itself.",
        "I think, therefore I am.",
        "The unexamined life is not worth living.",
        "To infinity and beyond!",
    ]
    return random.choice(quotes)


def random_tags(max_tags=5):
    """Generate a random list of tags."""
    num_tags = random.randint(0, max_tags)
    return random.sample(TAGS, num_tags)


def generate_random_url():
    """Generate a random URL using top domain names."""
    domain = random.choice(TOP_DOMAINS)
    path = "/" + random_string(5)
    return f"https://{domain}{path}"


def generate_random_snippet():
    """Generate a random snippet of text."""
    if random.choice([True, False]):
        return random_sentence()
    else:
        return random_quote()


# Main functions for generating and sending data
def send_url(url, tags):
    """Send a random URL to the server."""
    payload = {"url": url}
    response = requests.post(f"{BASE_URL}/urls/url", json=payload)
    print(f"URL Response ({url}): {response.status_code}")

    if tags:
        tags_payload = {"url": url, "tags": ",".join(tags)}
        response = requests.post(f"{BASE_URL}/urls/tags", json=tags_payload)
        print(f"Tags Response ({url}): {response.status_code}")


def send_snippet(url, snippet, tags):
    """Send a random snippet to the server."""
    payload = {
        "url": url,
        "snippet": snippet,
        "tags": ",".join(tags),  # Convert list to comma-separated string
    }
    response = requests.post(f"{BASE_URL}/snippets", json=payload)
    print(
        f"Snippet Response (Snippet: {snippet}): {response.status_code}, {response.text}"
    )


def generate_test_data(num_items):
    """Generate and send test data."""
    for _ in range(num_items):
        # Generate random data
        url = generate_random_url()
        tags = random_tags()
        snippet = generate_random_snippet()

        # Randomly decide whether to send URL, snippet, or both
        if random.choice([True, False]):
            send_url(url, tags)
        if random.choice([True, False]):
            send_snippet(url, snippet, tags)


# Run the script
if __name__ == "__main__":
    num_items = 1000  # Number of items to generate
    generate_test_data(num_items)
