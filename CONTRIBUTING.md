# Contributing

Thanks for taking the time to contribute. This repository is a simple Android
debloat script, and it is better for every bug report, suggestion, and patch
that comes in.

## Scope of Accepted Contributions

Contributions are welcome when they improve what this project is meant to do:

- Bug fixes and correctness improvements to the script and its package lists.
- Changes that make the debloat behavior safer, clearer, or better documented.
- Documentation fixes and clarifications.
- Tests, where the project has a test setup that fits them.

Unrelated rewrites, changes to the project's purpose, or large architectural
changes should be discussed in an issue before any code is written.

By participating in this project you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting Bugs

Before opening an issue, search the existing issues to check whether the bug has
already been reported. If it has, add your details to that issue instead of
opening a duplicate.

When you open a bug report, include:

- What you expected to happen, and what actually happened.
- The steps needed to reproduce the problem.
- Your environment: device or emulator, Android version, and the tool versions
  involved.
- The exact commands you ran and any relevant output or error messages.

Do not include secrets, account credentials, or any other private data in a bug
report.

## Suggesting Features

Open an issue describing the problem you want solved and the approach you have
in mind. Explain the use case and why the current behavior is not enough. Keep
the proposal scoped: a focused suggestion is easier to evaluate than a broad
redesign.

Security vulnerabilities must not be reported through public issues. See
[SECURITY.md](SECURITY.md) instead.

## Pull Request Workflow

1. **Fork** the repository and clone your fork.
2. **Branch** from the branch you intend to change, using a descriptive name
   such as `fix-package-list-parsing`.
3. **Commit** your work as small, self-contained logical units. See
   [`docs/GIT_GUIDELINES.md`](docs/GIT_GUIDELINES.md) for commit message and Git
   expectations.
4. **Open a pull request** against this repository. Describe the problem, the
   change you made, and how you verified it. Link any related issue.

Keep each pull request focused on one change. Unrelated cleanups make a change
harder to review and are usually asked to be split out.

Make sure your change behaves as expected before you open the pull request, and
run whatever checks the project provides. If a check could not be run, say so in
the pull request description rather than leaving it unmentioned.

## Coding and Design Standards

Follow [`docs/DESIGN_GUIDELINES.md`](docs/DESIGN_GUIDELINES.md). It covers
readability, simplicity, error handling, testing, documentation, dependency
discipline, configuration, and consistency expectations.

Where project-specific conventions, requirements, or instructions differ from
the general guidelines, the project-specific requirements take precedence.

## Commit and Git Practices

Follow [`docs/GIT_GUIDELINES.md`](docs/GIT_GUIDELINES.md). In short: work on a
branch, keep commits atomic, write clear imperative commit messages, avoid
rewriting shared history, and never commit secrets.

## License

By contributing, you agree that your contributions are accepted under the
repository's existing license. See [LICENSE](LICENSE) for the full text.
