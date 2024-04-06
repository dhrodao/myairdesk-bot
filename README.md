# Bertrandt MyAirDesk Bot

This is a bot developed in Rust over Ubuntu 22.04.4 LTS on Windows 10 x86_64 to automate bookings for the Bertrandt Hot Desks App.

## Usage

First of all you will need to get your airdesk Desk ID (AIRDESK_WORKPLACE). You will be able to get it if you inspect the network when booking your desk via [The Bertrandt Hot Desks App](https://bertrandt.myairdesk.com/).

This project needs some environment variables to be able to work:

```
export AIRDESK_USER=<your_user>
export AIRDESK_PASS=<your_pass>
export AIRDESK_WORKPLACE=<your_desk_id>
```

Finally, to run this project you need to run the following commands: 

```
$ git clone https://github.com/dhrodao/myairdesk-bot.git
$ cd myairdesk-bot
$ cargo run
```