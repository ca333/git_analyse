# Git Analyse

`git_analyse` is a rust-based command-line tool that analyzes a given GitHub or GitLab repository using OpenAI's GPT-3 model. The tool fetches the code from the default branch of the specified repository, processes it, and submits it to the GPT-3 API. It then generates a report on the purpose of the software, any suspicious behavior, and potential malware detection.

## Features

- Analyzes code from GitHub and GitLab repositories
- Uses OpenAI's GPT-3 model for in-depth code analysis
- Generates a report detailing the software's functionality and potential security risks

## Requirements

- Rust 1.54.0 or higher
- An OpenAI API key

## Installation

1. Clone this repository:

```bash
git clone https://github.com/your_username/git_analyse.git
```

2. Change to the git_analyse directory:

```bash
cd git_analyse
```

3. Build the project:

```bash
cargo build --release
```

4. Copy the binary to a directory in your PATH:
```bash
cp target/release/git_analyse ~/.local/bin/
```

## Setup

1. Create a .env file in the project's root directory:

```bash
touch .env
```

2. Add the following line to the .env file, replacing "your_api_key_here" with your OpenAI API key:

```
OPENAI_API_KEY=your_api_key_here
````

## Usage

1. Run the tool with a GitHub or GitLab repository URL as a parameter:
```bash
git_analyse https://github.com/username/repository.git
```

2. Review the generated report for insights into the software's purpose, suspicious behavior, and potential malware detection.

## Contributing

If you'd like to contribute to this project, feel free to submit a pull request with your proposed changes. Any contributions and suggestions are welcome!

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Disclaimer
This project was executed by OpenAI (GPT-4 language model), including this README.md

## Known Issues / TODOS
- fix gitlab binding
- add multi-language model support
- use gpt-4-32k model as default / https://platform.openai.com/docs/models/gpt-4
- create dependency tree / scan subdependencies
- generate threat model
- generate PDF report
- scan ref. CVE DBs API endpoint
