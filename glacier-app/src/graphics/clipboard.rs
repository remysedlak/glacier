/// TODO: implement piano roll and playlist clipboard
/// GOALS:
/// - copy paste midi on piano roll ; piano_roll.rs
/// - copy paste tracks and patterns ; playlist.rs
/// - copy and paste sequences ; sequencer.rs
enum Clipboard {
    Block(AudioBlockType, u32 /* length */),
    Notes(Vec<Note>),
    None,
}
