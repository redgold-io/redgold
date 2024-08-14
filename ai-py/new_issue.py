import json
import os
import requests


def create_github_issue(owner, repo, title, body, labels, token):
    url = f"https://api.github.com/repos/{owner}/{repo}/issues"
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.v3+json"
    }
    data = {
        "title": title,
        "body": body,
        "labels": labels
    }

    response = requests.post(url, headers=headers, json=data)

    if response.status_code == 201:
        issue = response.json()
        print(f"Issue created successfully. Issue number: {issue['number']}")
        return issue
    else:
        print(f"Error creating issue: {response.status_code}")
        print(response.text)
        return None


def create_issue_from_response(r: str):
    split = r.split("\n")
    split[0] = split[0].replace("```json", "")
    split[-1] = split[-1].replace("```", "")
    raw_json = "\n".join([k for k in split if len(k) > 0])
    print("raw_json: ", raw_json)
    j = json.loads(raw_json)
    title = j['title']
    body = j['body']
    tags = j['tags']
    owner = "redgold-io"
    repo = "redgold"
    labels = tags
    labels.append("ai-dev")
    token = os.environ['REDGOLD_AI_TOKEN']

    created_issue = create_github_issue(owner, repo, title, body, labels, token)
    if created_issue:
        print(f"Issue URL: {created_issue['html_url']}")
    else:
        print("Issue not created ", created_issue)


def main():
    with open("ignore-data/2024-08-11-15-50-34/text") as f:
        response = f.read()
        create_issue_from_response(response)


if __name__ == "__main__":
    main()
