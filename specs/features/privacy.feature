@RNF-03
Feature: Privacy: the app has no path to the Internet
  Acceptance: specs/contracts/privacy.md (rules T1 to F2, P1 to P6, M1).
  The rule scenarios are tests of scripts/lint-network.mjs through `just lint-arch`.
  Each violation is one line " - <file>: RNF-03 <rule>: <detail>" under "lint-arch: violations found", with exit code 1.

  Scenario: The repository as it is passes the privacy rules
    Given the repository at the candidate commit
    When `just lint-arch` runs
    Then it prints "lint-arch: ok" and exits with code 0

  Scenario Outline: A CSP source that can reach a remote origin is refused
    Given src-tauri/tauri.conf.json whose app.security.csp is "<csp>"
    When the privacy rules run
    Then the output has the line " - src-tauri/tauri.conf.json: RNF-03 T1: <detail>"

    Examples:
      | csp                                                                          | detail                                                |
      | default-src 'self'; connect-src ipc: http://ipc.localhost https://example.com | CSP source 'https://example.com' not allowed in connect-src |
      | default-src 'self'; img-src 'self' https:                                     | CSP source 'https:' not allowed in img-src             |
      | default-src *                                                                 | default-src must be 'self'                             |
      | default-src 'self'; script-src 'self' https://cdn.example.com                 | CSP source 'https://cdn.example.com' not allowed in script-src |
      | default-src 'self'; style-src 'self' http://ipc.localhost                     | CSP source 'http://ipc.localhost' not allowed in style-src |

  Scenario: A missing CSP is refused
    Given src-tauri/tauri.conf.json without app.security.csp
    When the privacy rules run
    Then the output has the line " - src-tauri/tauri.conf.json: RNF-03 T1: CSP missing"

  Scenario Outline: Tauri configuration that could reach the network is refused
    Given src-tauri/tauri.conf.json with "<json path>" set
    When the privacy rules run
    Then the output has the line " - src-tauri/tauri.conf.json: RNF-03 T2: '<json path>' not allowed"

    Examples:
      | json path                       |
      | plugins                         |
      | bundle.createUpdaterArtifacts   |
      | app.windows[0].url              |
      | app.security.capabilities       |

  Scenario Outline: A capability outside the allow-list is refused
    Given src-tauri/capabilities/default.json that also grants "<permission>"
    When the privacy rules run
    Then the output has the line " - src-tauri/capabilities/default.json: RNF-03 T3: permission '<permission>' not in the allow-list"

    Examples:
      | permission                               |
      | http:default                             |
      | shell:allow-open                         |
      | opener:default                           |
      | core:webview:allow-create-webview-window |

  Scenario: A capability open to remote origins is refused
    Given src-tauri/capabilities/default.json with a "remote" key listing "https://example.com"
    When the privacy rules run
    Then the output has the line " - src-tauri/capabilities/default.json: RNF-03 T3: remote key not allowed"

  Scenario: Tightening the surface is not a violation
    Given a CSP without the "data:" source in img-src
    And a capability whose permissions list is empty
    And a capabilities folder with no files
    When the privacy rules run on each
    Then none of them reports an RNF-03 violation

  Scenario Outline: A direct Rust dependency that can reach the network is refused
    Given <manifest> with the dependency line `<line>` in <table>
    When the privacy rules run
    Then the output has the line " - <manifest>: RNF-03 R1: <detail>"

    Examples:
      | manifest                            | table                 | line                                                         | detail                                                   |
      | crates/fakeflac-fs/Cargo.toml       | [dependencies]        | reqwest = "0.13"                                             | dependency 'reqwest' can reach the network               |
      | crates/fakeflac-store/Cargo.toml    | [dependencies]        | net = { package = "ureq", version = "3" }                    | dependency 'ureq' can reach the network                  |
      | tools/fakeflac-calibrate/Cargo.toml | [dev-dependencies]    | hyper = "1"                                                  | dependency 'hyper' can reach the network                 |
      | src-tauri/Cargo.toml                | [dependencies]        | tauri-plugin-clipboard-manager = "2"                         | tauri plugin 'tauri-plugin-clipboard-manager' not in the allow-list |
      | src-tauri/Cargo.toml                | [dependencies]        | tauri = { version = "2", features = ["rustls-tls"] }         | feature 'rustls-tls' of 'tauri' not allowed              |
      | src-tauri/Cargo.toml                | [target.'cfg(windows)'.dependencies] | windows = { version = "0.62", features = ["Win32_Networking_WinHttp"] } | feature 'Win32_Networking_WinHttp' of 'windows' not allowed |

  Scenario: A network crate in Cargo.lock is refused, and tauri's mobile-only crates are not
    Given a Cargo.lock with a [[package]] named "rustls"
    When the privacy rules run
    Then the output has the line " - Cargo.lock: RNF-03 R2: Cargo.lock contains 'rustls'"
    And the packages "reqwest", "hyper" and "hyper-util" of today's Cargo.lock give no R2 violation

  Scenario: The banned crates of deny.toml are the NET list
    Given deny.toml and scripts/lint-network.mjs
    Then the names in deny.toml [bans] deny equal the NET list of the module
    And CI's `cargo deny check` refuses any of them in the x86_64-pc-windows-msvc graph

  Scenario Outline: A socket API in shipped Rust source is refused
    Given <file> containing "<token>"
    When the privacy rules run
    Then the output has the line " - <file>: RNF-03 R4: '<token>' in shipped source"

    Examples:
      | file                               | token         |
      | crates/fakeflac-store/src/lib.rs   | std::net      |
      | src-tauri/src/run.rs               | UdpSocket     |
      | crates/fakeflac-fs/src/lib.rs      | TcpStream     |

  Scenario: Rust test files are not scanned for socket APIs
    Given crates/fakeflac-audio/tests/common/aac_tables.rs with URLs in comments
    When the privacy rules run
    Then no R4 violation names that file

  Scenario Outline: A network call or remote URL in the frontend is refused
    Given <file> containing `<text>`
    When the privacy rules run
    Then the output has the line " - <file>: RNF-03 F1: <detail>"

    Examples:
      | file                         | text                                                             | detail                                            |
      | src/lib/api/extra.ts         | fetch("https://example.com/a")                                   | URL 'https://example.com/a'                       |
      | src/lib/pages/Extra.svelte   | new WebSocket(url)                                               | 'WebSocket' not allowed                           |
      | src/lib/pages/Extra.svelte   | new EventSource("/events")                                       | 'EventSource' not allowed                         |
      | src/lib/pages/Extra.svelte   | navigator.sendBeacon(path, body)                                 | 'sendBeacon' not allowed                          |
      | index.html                   | <link rel="stylesheet" href="https://fonts.googleapis.com/css2"> | URL 'https://fonts.googleapis.com/css2'           |
      | src/lib/tokens/tokens.css    | @import url(//cdn.example.com/a.css);                            | URL '//cdn.example.com/a.css'                     |

  Scenario: The SVG namespace, tests and stories are not violations
    Given src/lib/components/atoms/Icon.svelte with xmlns="http://www.w3.org/2000/svg"
    And a file src/lib/api/extra.test.ts calling fetch("https://example.com")
    And a file under src/lib/testing/ with a URL
    When the privacy rules run
    Then no F1 violation is reported

  Scenario: A frontend Tauri plugin package is refused
    Given package.json with the devDependency "@tauri-apps/plugin-http"
    When the privacy rules run
    Then the output has the line " - package.json: RNF-03 F2: package '@tauri-apps/plugin-http' not allowed"

  @RNF-04
  Scenario Outline: An unreadable configuration file is a violation, not a crash
    Given <file> is <state>
    When `just lint-arch` runs
    Then the output has the line " - <file>: RNF-03 <rule>: <state>"
    And the other RNF-03 rules still report their own results
    And it exits with code 1 without a stack trace

    Examples:
      | file                                 | rule | state        |
      | src-tauri/tauri.conf.json            | T1   | invalid JSON |
      | src-tauri/tauri.conf.json            | T1   | missing      |
      | src-tauri/capabilities/default.json  | T3   | invalid JSON |
      | package.json                         | F2   | invalid JSON |

  @RNF-04
  Scenario: The installed app works with no network (manual M1, step 1)
    Given an installed release candidate whose version, commit and installer SHA-256 are recorded
    And every network adapter is disabled
    When I analyze the fixture folder with a valid file, a corrupt supported file and a valid file with a Unicode name
    Then each valid file has a real, suspicious or fake verdict and the corrupt file has an Error row
    When I open a spectrogram, export the CSV, analyze an empty folder, change and save the language, and clear the cache
    Then each step works and the empty folder shows NO_AUDIO_FOUND

  Scenario: The installed app opens no remote connection (manual M1, step 2)
    Given the network is enabled and Resource Monitor watches fakeflac.exe and its msedgewebview2.exe processes
    When I repeat the M1 session and leave the app idle for 5 minutes
    Then fakeflac.exe shows no non-loopback address in Network Activity or TCP Connections
    And any address of a WebView2 process is recorded with its host name as a finding for the owner
    And the record has aggregates only, with no audio names or paths
