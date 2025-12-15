# ECE1724 Final Project: LLM Inference Service 

## Presentation

Link: https://github.com/user-attachments/assets/2ca82d50-e430-407c-8f5e-cecb06ded242


## Demonstration

Link: https://github.com/user-attachments/assets/64287dd9-f1bc-47a1-8c1b-0e4a824e462f



## Team Information
| Member | Student Number | Email Address |
|---------|---------------------------|---------------------------|
| **Kimberly** |  1006012949 | kimberly.hau@mail.utoronto.ca |
| **Shafin** |  1006945406 | shafinul.haque@mail.utoronto.ca |
| **Alan** | 1007191316 | aloe.cao@mail.utoronto.ca |

## Motivation
This project idea is inspired by one of the ideas suggested on the course page: to build an LLM inference backend with streaming. We chose this project idea as LLMs have become widely used over the last few years, while still being relatively new to the Rust (and other programming languages) ecosystem. Therefore, we are interested in building a user application for LLM use which can build upon the currently available Rust inference engines such as Candle and Mistral.rs, and implement token-by-token streaming during inference, a feature that is currently unsupported by these engines.

This project gives us the opportunity to work with local large language models. We find this valuable as using LLMs have become an expected part of our research projects; however, it is not always possible or preferable to run our data through commercial LLMs such as ChatGPT or Claude, for reasons such as data privacy or costs. Therefore, it is helpful for us to be able to build our own applications to run local models, rather than relying on commercial LLMs. By building our own systems, we can deepen our understanding of these new emerging technologies.

In particular, we were motivated by the opportunity to better understand how local LLM inference servers manage shared resources when serving multiple users at the same time. LLM serving systems need to handle many concurrent requests on the same hardware, which requires careful management of compute, memory, and model state. State of the art inference systems such as vLLM demonstrate that separating the prefill and decode phases of generation and explicitly managing KV caches across requests can significantly improve throughput and latency in multi-user settings [1]. Building a local inference service allowed us to experiment with these ideas at a smaller scale, and to gain insight into how inference systems can serve multiple clients without dedicating a separate device or model instance to each user.

Finally, this project will allow us to work with an interesting tech stack and a variety of programming concepts, such as async programming, concurrency, memory safety, and networking in Rust.

## Objectives
To build a local, Rust-based LLM chat system that provides responses token-by-token to users in real time. We wanted LLM chat to feel responsive and interactive, similar to using ChatGPT or Claude, but running on our own machine. We also wanted to serve multiple users from a single inference backend.

**LLM Chat Application:**
- Provide a terminal-based chat interface with streaming responses.
- Maintain context across multiple messages within a chat.
- Support chat history, persistence, and resuming past conversations.
- Manage user sessions.

**Inference Service:**
- Run a single shared LLM instance for all clients.
- Stream tokens as they are generated over an HTTP API.
- Remain responsive under concurrent client requests
- Schedule inference fairly across multiple active chats

## Features
### 1. Inference Engine
- A backend engine that loads and runs the TinyLlama 1.1B chat model using the `candle` crate.
- Responsible for all model execution and token generation.
- Streams out generated tokens incrementally during inference.

### 2. Concurrent Request Handling

- The inference engine supports concurrent chat sessions using a single model instance.
- The engine manages per-request state (e.g., KV cache) for all active chats and fairly schedules token generation across them.
- LLM generation is separated 2 phases:
  - **Prefill:** processes new prompts sequentially to initialize KV cache
  - **Decode:** generates output tokens in a round-robin fashion across active chat sessions
- This approach allows multiple users to share one model while maintaining responsive, token-by-token streaming

### 3. Message Persistence and Chat History
- A SQLite database stores a list of users who have accessed the service.
    - For each user, all past messages in all past chats are stored to support retrieval and resumption of previous chats.
- A `/history` endpoint retrieves and lists a brief overview of a user's past conversations. 
- A `/fetch` endpoint exposes stored conversations over the HTTP API so clients can select and reload past chats.

### 4. Streaming LLM Inference Service API
- A JSON-based HTTP API implemented using the `axum` crate.
- Serves as the communication layer between the chat application (Terminal UI) and the backend inference engine and database.
- Endpoints include:
    - POST `/generate` for sending prompt and receiving back token-by-token model output
    - GET `/next_chat_id` for initializing a new chat session
    - GET `/history` for retrieving past conversations
    - GET `/fetch` for retrieving full chat transcripts
- Currently, the backend inference engine and database are spun up locally, so inference happens on the same machine as the user. However, the API design also supports hosting the backend remotely.

### 5. LLM Chat Terminal User Interface (TUI)
- A multi-screen terminal user interface built with the `ratatui` crate, allowing users to easily enter prompts and view LLM responses, as well as view and resume past conversations. Users can customize their experience through selecting the text colours of the chat.
- The TUI acts as a client of the Streaming Inference Service API, using it to send prompts, receive streaming token output, and retrieve stored chat history.
- The TUI includes multiple screens:
  - **Sign-in screen:** enter a username to start or resume a session.
  - **Main menu:** start new chats, view past chats, or change settings.
  - **Chatting screen:** type prompts and view streaming responses from the model.
  - **Chat history screen:** browse and reopen previous conversations.
  - **Text colour selection screen:** customize the appearance of the chat.

### Tech Stack

| Component | Technology | Purpose / Notes |
|------------|-------------|----------------|
| **Model Inference** | **Candle** | Load and run local LLM inference |
| **Web Framework** | **Axum** | Async web server and HTTP API with SSE streaming support |
| **Serialization** | **Serde / serde_json** | Handle JSON request/response formats |
| **Database** | **SQLite** | Store chat history and user sessions |
| **Frontend** | **Ratatui** | Interface for sending prompts and showing results |

## User's Guide

This section describes how a user can chat with the LLM through the terminal UI.

### Starting the System

1. Start the backend server:
   - In a terminal, navigate into the `llm-server` directory.
   - On macOS with hardware acceleration, run:
     ```bash
     cargo run --release --features accelerate
     ```
   - On other platforms or without acceleration, you can run:
     ```bash
     cargo run 
     ```
   - The first run will download model weights; this may take a few minutes depending on your connection.
   - Once the server is ready, you should see:
     `LLM streaming server listening on http://127.0.0.1:4000`

2. Start the terminal UI:
   - In a separate terminal, navigate into the `llm-ui` directory.
   - Run:
     ```bash
     cargo run
     ```

3. Ensure that the UI can connect to the server (by default, they communicate over `localhost` on the `4000` port).

### Using Main Features
1. Once the server has been started, all user interaction is on the TUI side, which will send prompts and receive responses from the server. The server schedules and handles all requests.
2. On the TUI, the application will first open to a Sign-in screen, where users can enter their name. This brings the user to the Main Menu, on which there are four buttons that users can select: start a new chat, resume a past chat, change text colour, or quit. All screens return to the Main Menu by pressing the **'ESC'** key.
    * **Main menu**: Navigate between buttons using **'Up'** and **'Down'** arrow keys, and confirm selection with **'Enter'**.
    * **Chat**: Press **'e'** to begin editing a new message request, and submit with **'Enter'**.
    * **Past history**: From the list of past chats, enter the corresponding numerical chat ID, and submit with **'Enter'**.
    * **Colour selection**: Navigate the colour options using using **'Up'** and **'Down'** arrow keys, and confirm selection for user messages' colour with **'Enter'**. Repeat to select colour for LLM server messages.
3. To observe concurrent inference behavior, launch multiple instances of the TUI in separate terminal windows and send messages simultaneously from different chats. The token output rate (`tok/s`) displayed at the top of the interface will decrease as more clients are active, but all clients should continue to receive generated tokens.

### Examples

## Reproducibility Guide
1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd <repository-root>
   ```
2. Start the backend inference server:
   ```bash
   cd llm-server
   ```
   - On macOS with hardware acceleration enabled:
   ```bash
   cargo run --release --features accelerate
   ```
   - On other platforms or without acceleration:
   ```bash
   cargo run
   ```
   The server is ready once it prints:
   ```bash
   LLM streaming server listening on http://127.0.0.1:4000
   ```
   On the first run, model weights will be downloaded automatically. This may take several minutes
3. Start the terminal user interface. In a separate terminal window:
   ```bash
   cd llm-ui
   cargo run
   ```
4. Once both components are running, users can sign in to the application and interact with the LLM using the features described in the User’s Guide.


## Individual Contributions
| Member | Contributions |
|---------|---------------------------|
| Shafin | Built streaming from Candle token generation back to clients and implemented concurrent inference scheduling with separate prefill and decode phases using round-robin decoding.  |
| Alan | Built SQLite database and integrated into UI through sign-in and past chat history screens. |
| Kimberly | Built main menu screen, chat input screen, colour selection screen for UI. |

## Lessons Learned
- Enums and pattern matching are useful for defining different states in which the system exists, greatly simplifying the design of the control flow.
- An overarching application state struct provides a centralized mechanism for effortless sharing of data between modules.
- How to use the features of Rust crates such as Ratatui and Axum
- When running inside an async runtime, it’s important not to run CPU-bound tasks directly, since they will block the runtime and reduce responsiveness. Model inference is CPU-bound, so we learned that it needs to run in a separate blocking thread, while the async runtime should only be responsible for short networking and handling tasks.
- Channels ended up being a simple way to communicate between the async server and the inference engine. We use one channel to send client requests into the inference thread, and per-request channels to send generated tokens back.
- Our experimentation with separating prefill and decode was not the most optimal. The dedicated prefill model is idle most of the time unless there are constantly large prompts or long contexts coming in. The idea of round-robin scheduling during decode worked well, but allowing both model instances to perform decode when there is no prefill work would likely have been a better use of resources.

## References
[1] Kwon et al., vLLM: Easy, Fast, and Cheap LLM Serving with PagedAttention, 2023.
[2] Candle: Rust-based ML Framework
[3] Mistral.rs
[4] Axum Web Framework
[5] Rusqlite: Rust bindings to SQLite
[6] Server-Sent Events

