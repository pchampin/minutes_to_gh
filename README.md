Minutes to github
=================

Yet another tool to link github issues to W3C minutes where they were discussed.

The goal is, eventually, to have an IRC bot that can be asked, at the end of a meeting,
to parse the minutes, extract link of topics or subtopic where a github issue was discussed,
and put a link in that issue to the corresponding discussion.

As an intermediate step, this will be achieved via a command line rather than an IRC bot.

Quick start
-----------

### On the command line (with Docker)

```bash
# prepare the Docker image (required only once)
docker build -t minutes_to_gh .
# run the command
docker run --rm -it minutes_to_gh --channel $IRC_CHANNEL --token $MY_GITHUB_TOKEN
```

This will fetch the minutes generated on the current date for the given IRC channel,
and add a link to those minutes in all github issues and pull requests discussed on the minutes
(assuming that the owner of the token has the permission to do so, of course).

* To test the application without actually posting comments on github, add the option `--dry-run`.
* To fetch the minutes from another day, add the option `--date [YYYY-MM-DD]`.
* To see more options, add the option `--help`.

Note that the program will not post a comment on an issue/pull-request where the same link to the minutes is already present in a comment.
So it is (relatively) safe to run the command several times with the same options.


### On the command line (with Cargo)

```bash
cargo run -- --channel $IRC_CHANNEL --token $MY_GITHUB_TOKEN
```

The same options as above are available.


### As an IRC bot

Coming soon...


Alternatives (with their cons)
------------------------------

* https://github.com/iherman/scribejs-postprocessing/

  - this requires minutes to be processed with [scribejs](https://github.com/w3c/scribejs)
    which is not the tool used by IRC bots such as RRSAgent

* https://github.com/dbaron/wgmeeting-github-ircbot/

  - pastes IRC log in github issues; no link to the HTML minutes
