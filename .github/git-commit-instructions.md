# Commit Message Instructions

When generating a Git commit message, follow these rules:

- Use the Conventional Commits specification.
- Use the format: `<type>(<scope>): <subject>`.
- Allowed types:
  - `feat`: introduce a new feature
  - `fix`: fix a bug
  - `refactor`: restructure code without changing behavior
  - `perf`: improve performance
  - `docs`: update documentation
  - `test`: add or update tests
  - `style`: change formatting without changing behavior
  - `build`: change build scripts or dependencies
  - `ci`: change CI configuration
  - `chore`: perform maintenance work
  - `revert`: revert an earlier commit
- Infer a short scope from the main affected module when appropriate.
- Write the subject in concise English.
- Use the imperative mood.
- Start the subject with a lowercase letter.
- Do not end the subject with a period.
- Keep the first line within 100 characters whenever possible.
- Describe the primary purpose of the change, not individual file operations.
- Do not mention filenames unless they are essential.
- Do not invent issue numbers, requirements, behavior, or motivations that are not visible in the changes.
- If the changes contain multiple unrelated concerns, describe the most important one in the subject and summarize the others in the body.
- Add a body only when the change requires additional explanation.
- Separate the subject and body with one blank line.
- Output only the commit message, without Markdown fences, headings, explanations, or alternatives.