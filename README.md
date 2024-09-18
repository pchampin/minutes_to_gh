Minutes to GitHub
=================

[Yet another](#alternatives-with-their-cons) tool to link GitHub issues to W3C minutes where they were discussed.

It is available in two flavours:

* an [IRC bot](#irc-bot-commands) that can be invoked right after the minutes have been generated, and
* a [command line tool](#manual-mode) that can add links to minutes generated in the past.

How it works
------------

This program fetches the minutes of a meeting (an HTML file),
find all mention to GitHub issues or pull requests,
and post a comment to each of them,
containing a link to the (sub)section where this issue/pull request was mentioned,
as well as, optionally, a copy of that section (converted to markdown).

Note that the program will not add a comment if it finds one already containing the same link,
so it should be safe to run it several times.

Quick start
-----------

Before you start, you need to create a [GitHub token](https://github.com/settings/tokens)
that will enable the program to post comments on GitHub on your behalf.

### Run the IRC bot with Docker

```bash
# prepare the Docker image (required only once)
docker build -t minutes_to_gh .
# run the IRC bot
docker run --rm --init -it minutes_to_gh --token $GITHUB_TOKEN irc-bot --username $YOUR_USERNAME
```

This will run an IRC bot named `m2gbot` that will connect to [`irc.w3.org`](https://irc.w3.org).
You can then invite it to any channel, and invoke it *after* the minutes have been generated, with
```
m2gbot, link issues to minutes
```
This will [process](#how-it-works) the minutes generated on the current day for the current IRC channel.

To see more available options, run:
```
docker run --rm -it minutes_to_gh help irc-bot
```


### Run the IRC bot with Cargo

```bash
cargo run -- --token $GITHUB_TOKEN irc-bot --username $YOUR_USERNAME
```

### Manual mode

For creating GitHub comments for older minutes, it is possible to use this program in "manual" mode.
The basic options are as follow:

```bash
docker run --rm -it minutes_to_gh --token $GITHUB_TOKEN manual --channel $IRC_CHANNEL --date $DATE
```
or
```bash
cargo run -- --token $GITHUB_TOKEN manual --channel $IRC_CHANNEL --date $DATE
```
where `$DATE` is formatted as `YYYY-MM-DD`.
This will [process](#how-it-works) the minutes generated on the current day for the current IRC channel.

To see more available options, run

```bash
docker run --rm -it minutes_to_gh help manual
```
or
```bash
cargo run -- help manual
```

IRC bot commands
----------------

The IRC bot supports the following commands (always preceded by `"<nickname>, "`).

<table>
  <tr>
    <td>
      <code>[please] link [github] issues [to minutes] [with transcript]</code>
    <td>
       <a href="#how-it-works">Process</a> the minutes of the current day for the current channel.
       If <code>with transcript</code> is used, the GitHub comments will include a copy of the relevant part of the minutes.
  <tr>
    <td>
      <code>debug</code>
    <td>
      Pretend to <a href="#how-it-works">process</a> the minutes as above, but do not actually post the comments.
  <tr>
    <td>
      <code>bye</code>, <code>[please] leave</code>
    <td>
      Leave the channel.
  <tr>
    <td>
      <code>[please] help</code>
    <td>
      Display a help message with the version of the program and a link to its homepage and documentation.
</table>


Alternatives (with their cons)
------------------------------

* https://github.com/iherman/scribejs-postprocessing/

  - this requires minutes to be processed with [scribejs](https://github.com/w3c/scribejs)
    which is not the tool used by W3C IRC bots such as [RRSAgent](https://www.w3.org/2002/03/RRSAgent)

* https://github.com/dbaron/wgmeeting-github-ircbot/

  - pastes raw IRC log in GitHub issues; no link to the HTML minutes
