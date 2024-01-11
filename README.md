# nimbus
macos utility tool intended to make organizing files easier for students.
currently, it only works for waterloo students but maybe in the future I will expand to cover other popular canadian unis.

## How it works:
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
