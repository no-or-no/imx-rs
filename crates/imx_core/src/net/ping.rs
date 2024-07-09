// todo last_ping_time 和 ping_time 是否只保留其一
#[derive(Debug, Default, Copy, Clone)]
pub struct Ping {
    pub(crate) last_check_ping_time: i64, // 单位: 毫秒
    pub(crate) last_ping_id: i64,
    pub(crate) ping_time: i32, // 单位: 秒
}

impl Ping {
    pub fn new() -> Self { Default::default() }
}

/*pub fn check_run(session: Session) -> Option<NetworkMessage> {
    let mut state = session.get_ping_state();
    let now = TimeSync::local_millis();
    if now - state.last_ping_time < PING_DURATION {
        return None;
    }

    state.last_ping_time = now;
    state.last_ping_id += 1;
    state.ping_time = TimeSync::local_seconds();
    session.set_ping_state(state);

    let mut req = PingDelayDisconnect::default();
    req.ping_id = state.last_ping_id;
    req.disconnect_delay = 35;

    // let body = proto::to_bytes(&req).unwrap_or_default();
    // let mut msg = Message::default();
    //
    // msg.msg_id = session.new_msg_id();
    // msg.bytes = body.len() as i32;
    // msg.body = Bytes::from(body);
    // todo
    // msg.seqno = conn.generate_msg_seq_no(false) as i32;

    Some(NetworkMessage::new(msg))
}*/
