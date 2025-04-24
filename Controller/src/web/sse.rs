use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct Event {
    pub id: Option<Cow<'static, str>>,
    pub data: Cow<'static, str>,
}
impl Event {
    pub fn to_payload(&self) -> String {
        let mut buffer = String::new();

        if let Some(id) = &self.id {
            if id.is_empty() {
                buffer.push_str("id: \r\n");
            } else {
                id.lines().for_each(|line| {
                    buffer.push_str("id: ");
                    buffer.push_str(line);
                    buffer.push_str("\r\n");
                });
            }
        }
        if self.data.is_empty() {
            buffer.push_str("data: \r\n");
        } else {
            self.data.lines().for_each(|line| {
                buffer.push_str("data: ");
                buffer.push_str(line);
                buffer.push_str("\r\n");
            });
        }
        buffer.push_str("\r\n");

        buffer
    }
}
