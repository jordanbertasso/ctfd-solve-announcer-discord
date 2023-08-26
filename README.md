# ctfd-solve-announcer-discord

A simple webhook based Discord bot to announce CTFd solves

## Usage

Install with

```bash
cargo install --git https://github.com/jordanbertasso/ctfd-solve-announcer-discord
```

Run with

```bash
ctfd-solve-announcer-discord --help
```

## Dockerfile Usage

Build the docker image

```bash
docker build --tag bot .
```

Run a container using the created image and provide the environment variables

```bash
docker run -d --name ctfd-solve-announcer-discord \
    -e WEBHOOK_URL=<YOUR_WEBHOOK_URL> \
    -e CTFD_URL=<YOUR_CTFD_APP_URL> \
    -e CTFD_API_KEY=<YOUR_CTFD_API_KEY>
```

Enjoy!

## Contributions

Welcome! :D
