# ECE1724 Course Project: LLM Inference Streaming Application

## Motivation
This project idea is inspired by one of the ideas suggested on the course page: to build an LLM inference backend with streaming. We chose this project idea as LLMs have become widely used over the last few years, while still being relatively new to the Rust (and other programming languages) ecosystem. Therefore, we are interested in building a user application for LLM use which can build upon the currently available Rust inference engines such as Candle and Mistral.rs, and implement token-by-token streaming during inference, a feature that is currently unsupported by these engines. 

This project gives us the opportunity to work with local large language models. We find this valuable as using LLMs have become an expected part of our research projects; however, it is not always possible or preferable to run our data through commercial LLMs such as GPT or Claude, for reasons such as data privacy or costs. Therefore, it is helpful for us to be able to build our own applications to run local models, rather than relying on commercial LLMs. By building our own systems, we can deepen our understanding of these new emerging technologies. Finally, this project will allow us to work with an interesting tech stack and a variety of programming concepts, such as async programming, concurrency, memory safety, and networking in Rust.

---

## Objective and Key Features

### Objective
To design and implement a lightweight LLM inference service that supports **streaming outputs** token-by-token to clients via HTTP. The service will allow users to select and load a local model, accept prompts through an API, and stream generated text incrementally, simulating the responsiveness of modern LLM chat systems.

### Key Features

#### Core Features
1. **Model Selection, Loading, and Management**
   - Allow user selection from list of available models.
   - Load a small local model (e.g., using Candle) for inference.
   - Implement a lightweight model manager to initialize and reuse models efficiently.

2. **Streaming Inference**
   - Modify the model’s generation loop to emit tokens incrementally instead of waiting for full output.
   - Use **Axum** and **Server-Sent Events (SSE)** to stream tokens to the client in real time.

3. **Client Interaction**
   - Build a simple web or CLI interface for sending prompts and viewing streamed responses.
   - Provide basic input/output handling and connection to the backend API.
   - Provide model selection from list.

#### Stretch Goals
4. **Chat History and Persistence**
   - Use SQLite with SQLx to store past chat sessions and message history.
   - Allow retrieval of past chats and reuse of context within a session.

5. **Multi-User Support**
   - Enable multiple clients to connect simultaneously.
   - Leverage Axum’s async runtime to handle multiple requests.

6. **Naive Request Queuing**
   - Implement a simple FIFO queue to manage simultaneous user prompts.

#### Advanced / Exploratory Ideas
- Improve concurrency and scheduling (e.g., round-robin prompt token generation).
- Cache inference state (KV cache reuse) for faster context handling in long conversations.
- Add optional voice-to-text or image-to-text input.

---

### Tech Stack

| Component | Technology | Purpose / Notes |
|------------|-------------|----------------|
| **Model Inference** | **Candle** | Load and run local LLM inference |
| **Web Framework** | **Axum** | Async web server and HTTP API with SSE streaming support |
| **Serialization** | **Serde / serde_json** | Handle JSON request/response formats |
| **Database** | **SQLite + SQLx** | Store chat history and user sessions |
| **Frontend** | **Simple HTML/JS** or **Rust CLI** | Interface for sending prompts and showing results |

---

### Work Division

| Member | Responsibilities |
|---------|------------------|
| **Shafin** | **Phase 1:** Work with Alan on integrating Candle with Axum to enable SSE token streaming.<br>**Phase 2:** Develop the frontend (web or CLI) for sending prompts and displaying streamed responses; add basic chat continuation support.<br> |
| **Alan** | **Phase 1:** Work with Shafin on backend implementation of the streaming pipeline between Candle and Axum.<br>**Phase 2:** Implement chat history and persistence using SQLite with SQLx.
| **Kimberly** | **Phase 1:** Design the API structure and message flow between client and backend.<br>**Phase 3:** Implement server-side handling of concurrent requests and queuing for multiple clients. |

_All members will contribute to debugging, testing, and documentation as needed._

Workload distribution may shift as new challenges come up or development priorities evolve.

---

## Tentative Plan

Our plan is arranged in phases that allows us to start simple and expand as we gain more familiarity with Rust.

Since all group members are new to Rust, we don’t yet have a strong sense of how time-consuming debugging, ownership issues, or async handling might be. To manage this uncertainty, we’ll first aim to build the **core streaming feature** and ensure it’s functional, which will be our Minimum Viable Product. Afterwards, we can supplement the application by branching out into additional features like chat history, concurrency, and multi-user support.

### Phase 1: Core Streaming Feature
- Shafin and Alan work together to integrate the model backend (Candle) with Axum.
- Implement Server-Sent Events (SSE) to stream generated tokens to the client.
- Verify that prompts sent through the API produce real-time streamed responses.
- Kimberly assists by setting up basic project scaffolding and API routes for testing.

### Phase 2: Chat Interface and Persistence
- Add a simple frontend or CLI client to interact with the backend.
- Implement SQLite storage using SQLx to persist chats and restore previous sessions.
- Add support for maintaining context within a single chat (feed previous messages into new prompts).
- Begin integration testing to verify data flows across components.

### Phase 3: Multi-User and Optional Extensions
- Introduce concurrency handling for multiple users and queued requests.
- Expand the backend to manage user sessions and separate chat histories.
- Explore optional stretch goals such as multiple model support, keyword search, or simple authentication.
- Wrap up with testing, documentation, and video demo preparation.

We aim to begin writing our final report following the completion of Phase 2, and update as the project progresses. As Phase 3 involves the development of our stretch goals, we will work on this phase until 1-2 weeks before the deadline, before pivoting to drafting our slides, writing a script and filming our video demonstration.

---

### Feasibility
The project scope is intentionally kept manageable, with each feature designed to be independent and incrementally implemented.  
The core version (streaming inference, API, and basic chat handling) should be achievable within 3–4 weeks of part-time work and will have the focus of all three members. Stretch goals can be explored once the minimum viable product is complete.  

This phased plan ensures that even if we face unexpected challenges while learning Rust, we can still deliver a working system that meets the course requirements while leaving room for extension if progress allows. As well, it ensures that we will have enough time to properly develop the final report and video slides and demonstration.

---

## References
- [Candle: Rust-based ML Framework](https://github.com/huggingface/candle)
- [Mistral.rs](https://github.com/EricLBuehler/mistral.rs)
- [Axum Web Framework](https://docs.rs/axum/)
- [SQLx Async Database](https://docs.rs/sqlx/)
- [Server-Sent Events (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
