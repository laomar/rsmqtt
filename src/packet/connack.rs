#[derive(Debug, Default)]
pub struct ConnAck {
    session_present: bool,
    reason_code: u8,
    properties: Option<ConnAckProperties>,
}

impl ConnAck {}

#[derive(Debug, Default)]
pub struct ConnAckProperties {}