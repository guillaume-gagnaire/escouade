//! The five voice services on line as plugins (`dev.escouade.openai`, `.groq`, `.mistral`, `.elevenlabs`, `.deepgram`;
//! INT-13): each is this program started with the name of its service, and runs the service's side of `voice/*`.

macro_rules! service {
    ($name:ident, $id:literal) => {
        pub fn $name() -> i32 {
            escouade_lib::voice_plugin($id)
        }
    };
}

service!(openai, "openai");
service!(groq, "groq");
service!(elevenlabs, "elevenlabs");
service!(deepgram, "deepgram");

/// Mistral: its voice and, in the same process, Mistral Vibe (INT-10.6 bis, docs/PLUGINS.md §4.13).
pub fn mistral() -> i32 {
    escouade_lib::voice_plugin_with(
        "mistral",
        escouade_plugin_mistral::capabilities(),
        escouade_plugin_mistral::handler(),
    )
}
