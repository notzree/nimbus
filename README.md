# nimbus
macos utility tool intended to make organizing files easier for students.
currently, it only works for waterloo students but maybe in the future I will expand to cover other popular canadian unis.

## How it works:
Nimbus is a daemon that runs under Launchd. You can think of LaunchD as how MacOS manages cron jobs and scripts. This allows Nimbus to continuosly monitor your downloads folder behind the scenes. When Nimbus detects a downloaded file that matches one of the courses you are currently taking, it will save a suggestion to either move the file, do nothing, or if the file was indeterminate. Then the user can can run a seperate command to review and either accept or drop the suggestion. 


After cloning the repo, you can either run the application with launchd or with cargo.
#### LaunchD instructions:
First move the .plist file into ~/Library/LaunchAgents/. If you are on MacOS you can run this command:
```
mv ca.richard-zhang.nimbusMonitor.plist ~/Library/LaunchAgents/ca.richard-zhang.nimbusMonitor.plist
```
Then, to start nimbus run:
```
launchctl load ~/Library/LaunchAgents/ca.richard-zhang.nimbusMonitor.plist
```


#### Cargo instructions:
Run these commands in order and follow the prompts!
```
cargo run -- config
cargo run -- start
# when you want to review any commands
cargo run -- review
```

## What I've learned:
This is one of my first Rust projects and this has made me fall in love with Rust even more. I love the typing system way more than Typescript, the enums are intuitive and the error handling is amazing. I'm admittedly still fixing my bad habits when it comes to explicit error handling, but writing Rust code has made the quality of my code in other languages improve as well. Aside from Rust, I've learned a bit about how LaunchD works and what daemons are. Definitely would like to try writing more native apps down the line (with actual UIs).  
