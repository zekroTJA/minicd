# MiniCD

A tiny, simple CI/CD server for Git repositories.

## Why?

First of all, I wanted to have a very simple solution to automatically build and deploy 
micro-projects which won't be published on GitHub. Also, I really did not want to set up a complex
CI/CD system like Jenkins, Concourse or whatever else.

Also, I wanted to dive a bit deeper into how to build automation systems myself. So this was
a great practice to do so.

## Setup

You can get the latest binaries from the [releases page](https://github.com/zekrotja/minicd/releases).
Alternatively, you can compile it by yourself using cargo install.
```
cargo install --git https://github.com/zekrotja/minicd
```

### Configuration

MiniCD will look for a config file either in `./minicd.toml` or `/etc/minicd/config.toml` (in that order). If both configs exist, both will merge together in the read in order. You can also specify the config as `.yaml` file.

Here you can see an example configuration.
```toml
# The HTTP port of the API.
port = 8080
# The directory where your repositories are on your server.
# These repos will automatically get injected the post_receive
# hook to trigger jobs in minicd.
# If not specified, no repos will be injected automatically.
repo_dir = "/home/git/repos"
# The interval (in seconds) in which repositories in the repo_dir 
# will be scanned for new repositories.
index_interval_secs = 30
# A file containing secret values which will be injected into jobs.
secrets_file = "/root/secrets.yaml"

# Mail configuration for e-mail notifications.
[email]
# The SMTP server address.
smtp_server = "smtps.example.com"
# SMTP username.
username = "example"
# SMTP password.
password = "3x4mpl3"
# The e-mail addres from which notification e-mails will be sent.
from_address = "minicd@example.com"
```

## Remote Repository Setup

After setting up MiniCD on your server, simply create your bare Git remote repositories in the configured directory. After that, MiniCD will look for new repositories in the configured interval and inject the necessary `post_receive` hook to execute jobs.

## Project Setup

On the project side, simply create a file called `.minicd` in your repositories root directory. The configuration is defined using YAML.

You can use the provided schema file for better integration with your IDE. Simply add this line at the top of the `.minicd` file.
```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/zekroTJA/minicd/main/docs/schemas/.minicd.schema.json
```

Below, you can find an example configuration.
```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/zekroTJA/minicd/main/docs/schemas/.minicd.schema.json

name: "My Repo"

jobs:
  test:
    on:
      tag: ".*"
    notify:
      - on: [finish]
        to:
          - type: webhook
            url: https://example.com
          - type: email
            address: "{{notifications.email}}"
    shell: /bin/bash
    run: |
      docker build . -t myapp:latest
      docker login -u $SECRETS_DOCKER_USERNAME -p $SECRETS_DOCKER_PASSWORD
      docker push myapp:latest
```

As you can see, values form the configurated secrets file are injected into definition values in the format of `{{<key>}}`. In the run script, all secrets are passed in via environment variables with canonicalized keys in the format `SECRETS_<key>` where section delimiters are replaced by underscores (`_`) and all characters are uppercased.