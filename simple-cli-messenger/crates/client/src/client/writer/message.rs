use protocol::client_message::ClientMessageReceiver;

pub enum ClientWriterActorMessage {
    SendData(String, ClientMessageReceiver),
    Keepalive(),
}
