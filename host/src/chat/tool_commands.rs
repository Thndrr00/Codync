//! The `/n8n …`, `/comfy …`, `/image …` and `/video …` commands of the chat box: a message that
//! starts with one asks the bot to do its request with that tool. Like a skill's command, the host
//! puts a note before the text; the bot works n8n through the connector the host registers while
//! the instance runs, makes images with the `codync-comfyui` tools, and makes pictures and clips
//! with the image tools of its own agent (`media::command`). A skill with the same id wins
//! (`market::expand_command` goes first).

use crate::market::slash_command;

/// A command that hands the request to one of the computer's tools: what follows the `/`, and the note.
struct ToolCommand {
    name: &'static str,
    note: &'static str,
}

const COMMANDS: [ToolCommand; 2] = [
    ToolCommand {
        name: "n8n",
        note: "[Codync: the user asks you to do this with n8n, the workflow tool on this computer. Use the n8n \
               connector's tools: look at the workflows, run one or follow its executions, as the request needs. \
               If the connector isn't there, n8n isn't running: say so and tell them to start it in Settings → n8n.]",
    },
    ToolCommand {
        name: "comfy",
        note: "[Codync: the user asks you to make this with ComfyUI, the image tool on this computer. Use the \
               codync-comfyui tools: list_models first if you need a checkpoint, then queue the image and give the \
               result back. If the tools aren't there, ComfyUI isn't running: say so and tell them to start it from \
               its page in the sidebar.]",
    },
];

/// The commands the clients list after the skills: `(name, title, detail)`; the clients translate the words.
pub const LISTED: [(&str, &str, &str); 4] = [
    ("image", "Image", "Make an image with your agent's subscription"),
    ("video", "Video", "Make a short video with your agent's subscription"),
    ("n8n", "n8n", "Do it with the computer's n8n workflows"),
    ("comfy", "ComfyUI", "Make an image with ComfyUI"),
];

/// `text` with the tool's note before the request, when it starts with one of the commands; `None`
/// sends the message as written. `backend` is the asked bot's agent, which decides the image and
/// video models: a backend without image tools answers a note saying so.
pub fn expand(text: &str, backend: &str) -> Option<String> {
    if let Some(asked) = crate::media::command::expand(text, backend) {
        return Some(asked);
    }
    let (name, rest) = slash_command(text)?;
    let command = COMMANDS.iter().find(|c| c.name == name)?;
    Some(match rest.map(str::trim).filter(|r| !r.is_empty()) {
        Some(task) => format!("{}\n\n{task}", command.note),
        None => command.note.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_command_puts_its_note_before_the_request() {
        let asked = expand("/n8n run the daily report", "claude").expect("n8n command");
        assert!(asked.starts_with("[Codync:") && asked.contains("n8n connector"));
        assert!(asked.ends_with("\n\nrun the daily report"));
        let image = expand("/comfy a red fox\nin the snow", "claude").expect("comfy command");
        assert!(image.contains("codync-comfyui") && image.ends_with("a red fox\nin the snow"));
    }

    #[test]
    fn a_command_alone_is_just_its_note_and_other_text_is_left_alone() {
        assert!(expand("/n8n", "claude").expect("bare command").ends_with(']'));
        assert!(expand("/comfy   ", "claude").expect("blank request").ends_with(']'));
        assert_eq!(expand("hello /n8n", "claude"), None);
        assert_eq!(expand("/fast", "claude"), None);
        assert_eq!(expand("/n8nx do it", "claude"), None, "only the exact name");
    }

    #[test]
    fn an_image_command_goes_to_the_agents_own_tool() {
        let asked = expand("/image a red fox", "grok").expect("image command");
        assert!(asked.contains("image_gen") && asked.contains("a red fox"));
        let refused = expand("/video a red fox", "claude").expect("video command");
        assert!(refused.contains("cannot make a video"), "{refused}");
    }

    #[test]
    fn every_listed_command_expands() {
        let backend = |name: &str| if matches!(name, "image" | "video") { "grok" } else { "claude" };
        for (name, ..) in LISTED {
            assert!(expand(&format!("/{name} go"), backend(name)).is_some(), "{name}");
        }
    }
}
