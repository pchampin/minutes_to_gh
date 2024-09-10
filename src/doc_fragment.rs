/// Combines an ID from a DOM tree with the HTML fragment "accessible" from this ID.
pub struct DocFragment<'a> {
    pub id: &'a str,
    pub content: String,
}
