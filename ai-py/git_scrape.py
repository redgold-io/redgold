import requests
import json
from datetime import datetime


def get_github_issues(owner, repo):
    url = f"https://api.github.com/repos/{owner}/{repo}/issues"
    issues = []
    page = 1

    while True:
        response = requests.get(f"{url}?page={page}&per_page=100")
        if response.status_code == 200:
            page_issues = response.json()
            if not page_issues:
                break
            issues.extend(page_issues)
            page += 1
        else:
            print(f"Error: {response.status_code}")
            break

    return issues


def format_issues(issues):
    formatted_issues = []
    for issue in issues:
        labels = " ".join([f"#{label['name']}" for label in issue['labels']])
        created_at = datetime.strptime(issue['created_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")
        updated_at = datetime.strptime(issue['updated_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")

        formatted_issue = (
            f"{issue['state']} {issue['number']} - {issue['title']} - {labels}\n"
            f"{issue['body']}\n"
            f"created_at: {created_at}, updated_at: {updated_at}\n"
        )
        formatted_issues.append(formatted_issue)
    return formatted_issues


def all_issues_one_string():
    owner = "redgold-io"
    repo = "redgold"

    issues = get_github_issues(owner, repo)
    formatted_issues = format_issues(issues)
    return '\n'.join(formatted_issues)


def main():
    print(all_issues_one_string())


if __name__ == "__main__":
    main()