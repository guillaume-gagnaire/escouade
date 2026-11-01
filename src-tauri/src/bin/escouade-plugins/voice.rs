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
service!(mistral, "mistral");
service!(elevenlabs, "elevenlabs");
service!(deepgram, "deepgram");
