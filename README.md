# Soundboard

A free and easy-to-use program to organize and play sound files.



## How to Install

Head to the [releases](https://github.com/r-o-b-o-t-o/soundboard/releases) page, download the latest archive for your platform and extract to the directory of your choice.



## How to use

* The application consists of two windows, a **Settings** window and a **Soundboard** window.
* The Settings window allows you to configure the application and manage your sounds library, and will open when you start the application. To open the Settings window after closing it, right-click on the **Soundboard tray icon** in the taskbar notification area, and then click on "**Settings**".
* The Soundboard window is where you will be able to click on boxes to play the corresponding sound. To open the Soundboard window, right-click on the **Soundboard tray icon** in the taskbar notification area, and then click on "**Soundboard**", or directly left-click on the **Soundboard tray icon** in the taskbar notification area. You can also quickly open the Soundboard window by using the global shortcut <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Space</kbd>. Press <kbd>Escape</kbd> to close the Soundboard window.
* To completely quit the application, right-click on the Soundboard tray icon in the taskbar notification area, and then click on "**Quit**".

### How to use with Voice Chat applications

* Windows: [how to use with voice chat applications on Windows](readme/how-to-use-with-vc-apps-windows.md)



## Getting Started (Developer Guide)

### Prerequisites

* [Prerequisites on Windows](readme/prerequisites-windows.md)

* Cargo script
Install `cargo-run-script`:
```sh
user@machine:~$ cargo install cargo-run-script
```

* Install [Node.js](https://nodejs.org/en/download/)

### Installing

Create a clone of this project on your development machine:
```sh
user@machine:~$ git clone https://github.com/r-o-b-o-t-o/soundboard.git
```

Build the webview:
```sh
user@machine:~/soundboard$ cd soundboard-ui/
user@machine:~/soundboard/soundboard-ui$ npm i # you need to do this only once
user@machine:~/soundboard/soundboard-ui$ npm start
```
If you want to rebuild the webview automatically when doing changes:
```sh
user@machine:~/soundboard$ cd soundboard-ui/
user@machine:~/soundboard/soundboard-ui$ npm i -g nodemon # you need to do this only once
user@machine:~/soundboard/soundboard-ui$ nodemon
```

Run the program:
```sh
user@machine:~/soundboard$ cargo run
```

To package a release, use the following command:
```sh
user@machine:~/soundboard$ cargo run-script package
```



## Authors

* [r-o-b-o-t-o](https://github.com/r-o-b-o-t-o)

See also the list of [contributors](https://github.com/r-o-b-o-t-o/soundboard/graphs/contributors) who participated in this project.
