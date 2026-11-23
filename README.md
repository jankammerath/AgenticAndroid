# Agentic Android

Agentic Android is an app built in Rust with a minimal Kotlin app frontend, and serving a webserver with a web frontend. It uses Android's accessibility features to control the phone, and exposes the accessbility features to the configured agent to control the phone. This includes tap, swipe, key presses, and any other interaction a human can do on the phone.

![Agentic Android doing price comparison](screenshots/agenticandroid-shopping.png)

## Core Component

The core components of the app use NDK libraries written in Rust so ensure maximum performance. The Kotlin/JVM codebase is kept to the bare minimum and only used when absolutely necessary.

## App UI

The basic App UI only exists to configure the webserver, which is the main user interface of the app. Hence, the app is mainly remote controlled over the web ui, and runs in the background. 

## Web UI

The web ui mainly consists of a chatbot interface next to a live stream of the device screen and audio. There is a settings screen in which the user can configure the app. The user can configure it to use Claude, Gemini, Grok, or OpenAI by entering the respective API key for the said service. Multiple AI keys can be entered, and for each session, the user can decide which provider and which model to use.