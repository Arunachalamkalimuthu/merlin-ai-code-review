//! Agent channels — pluggable input/output adapters.
//!
//! Each channel implements `AgentChannel` from the parent module:
//!
//! | Channel  | Trigger                               | Setup                              |
//! |----------|---------------------------------------|------------------------------------|
//! | `cli`    | Stdin/stdout REPL                     | none                               |
//! | `slack`  | Slack app mention / DM                | SLACK_BOT_TOKEN, SLACK_SIGNING_SECRET |
//! | `discord`| Discord bot mention / DM              | DISCORD_BOT_TOKEN, DISCORD_CHANNEL_IDS |

pub mod cli;
pub mod discord;
pub mod slack;

pub use cli::CliChannel;
pub use discord::DiscordChannel;
pub use slack::SlackChannel;
