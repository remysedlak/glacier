enum Clipboard {
    Block(AudioBlockType, u32 /* length */),
    Notes(Vec<Note>),
    None,
}
