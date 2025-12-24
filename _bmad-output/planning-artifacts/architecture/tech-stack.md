# Tech Stack

|Category|Technology|Version|Purpose|Rationale|
|---|---|---|---|---|
|**Frontend Language**|TypeScript|`~5.4.5`|Language for UI development|Provides strong typing to reduce errors and improve maintainability.|
|**Frontend Framework**|React|`~18.3.1`|UI library for building components|Robust ecosystem, excellent performance, and pairs well with Tauri.|
|**UI Component Lib**|Ant Design (AntD)|`~5.17.4`|Pre-built UI components|Provides a professional, data-dense look and feel out of the box, accelerating development.|
|**State Management**|Zustand|`~4.5.2`|Manages UI state|A simple, lightweight, and unopinionated state management solution that avoids boilerplate.|
|**Backend Language**|Rust|`~1.78.0`|Core application logic, security|Guarantees memory safety and world-class performance, ideal for a security-critical app.|
|**App Framework**|Tauri|`~2.0.0-beta`|Cross-platform desktop app shell|Unifies Rust backend and web frontend into a small, secure, and fast native binary.|
|**API Style**|Tauri IPC / REST|`N/A`|FE/BE Communication|Tauri's Inter-Process Communication for the desktop app; REST for the optional cloud sync.|
|**Database**|SQLite|`~3.45.3`|Local, embedded data storage|A serverless, self-contained, and reliable database perfect for offline desktop applications.|
|**DB Access (Rust)**|`rusqlite` crate|`~0.31.0`|Rust interface for SQLite|Provides a safe and idiomatic way to interact with the SQLite database from the Rust core.|
|**Password Hashing**|`bcrypt` crate|`~0.15.1`|Securely hash user passwords|Industry-standard library for securing user credentials at rest.|
|**Frontend Testing**|Vitest|`~1.6.0`|Unit & Integration testing for UI|Modern, fast, and Jest-compatible test runner that integrates seamlessly with Vite.|
|**Backend Testing**|Rust Test Suite|`(built-in)`|Unit & Integration testing for core|Rust's powerful, built-in testing capabilities are sufficient and idiomatic.|
|**IaC Tool**|AWS CDK|`~2.144.0`|Infrastructure as Code for AWS|Define cloud infrastructure programmatically in TypeScript for reliability and repeatability.|
|**CI / CD**|GitHub Actions|`N/A`|Automated builds, tests, releases|Ubiquitous, powerful, and well-integrated with source control.|
|**Monitoring**|AWS CloudWatch|`N/A`|Monitor serverless sync functions|Native AWS solution for logging and monitoring the optional backend.|
|**Logging (Rust)**|`tracing` crate|`~0.1.40`|Structured application logging|A modern and powerful logging framework for Rust applications.|