use tokio::io::{AsyncRead, AsyncWrite};
use std::io::{Error, ErrorKind, Result};
use tokio::net::{TcpStream, UdpSocket};
use std::sync::{Arc};
use once_cell::sync::OnceCell;
use crate::protocol::trojan;
use crate::protocol::common::request::{InboundRequest, TransportProtocol};
use crate::protocol::common::stream::{StandardTcpStream};
use crate::protocol::trojan::packet::{
    TrojanPacketReader,
    TrojanPacketWriter,
};

static TCP_HANDLER: OnceCell<TcpHandler> = OnceCell::new();

pub struct TcpHandler {
}

impl TcpHandler {
    pub fn init() -> &'static TcpHandler {
        TCP_HANDLER.get_or_init(|| Self {

        })
    }

    pub async fn dispatch<T: AsyncRead + AsyncWrite + Unpin + Send>(
        &self,
        inbound_stream: StandardTcpStream<T>,
        request: InboundRequest,
    ) -> Result<(u64, u64)> {
        match request.transport_protocol {
            TransportProtocol::TCP =>  return  self.handle_direct_tcp(request, inbound_stream).await,
            TransportProtocol::UDP => return self.handle_direct_udp(request, inbound_stream).await,
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "unsupported protocol",
             )),
        }

    }

    async fn handle_direct_tcp<T: AsyncRead + AsyncWrite + Unpin + Send>(
        &self,
        request: InboundRequest,
        inbound_stream: StandardTcpStream<T>,
    ) -> Result<(u64, u64)> {

        let addr = request.into_destination_address();

        let outbound_stream = match TcpStream::connect(addr).await {
            Ok(stream) => stream,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::ConnectionRefused,
                    format!("failed to connect to tcp {}: {}", addr, e),
                ))
            }
        };
        let (mut source_read, mut source_write) = tokio::io::split(inbound_stream);
        let (mut target_read, mut target_write) = tokio::io::split(outbound_stream);

        return match tokio::join!(
            tokio::io::copy(&mut source_read, &mut target_write),
            tokio::io::copy(&mut target_read, &mut source_write),
        ){
            (Ok(u_size), Err(_))=> {
                Ok((u_size, 0))
            },
            (Err(_), Ok(d_size))  => {
                Ok((0, d_size))
            } ,
            (Ok(u_size), Ok(d_size)) => {
                Ok((u_size, d_size))
            }
            (Err(e1), Err(e2)) => {
                Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("copy error{}: {}", e1, e2),
                ))
            }
        }
    }



    #[allow(dead_code)]
    async fn handle_direct_udp<T: AsyncRead + AsyncWrite + Unpin + Send>(
        &self,
        request: InboundRequest,
        inbound_stream:StandardTcpStream<T>
    ) -> Result<(u64, u64)> {
        let addr = request.into_destination_address();
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        match socket.connect(addr).await {
            Ok(_) => (),
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::ConnectionRefused,
                    format!("failed to connect to udp {}: {}", addr, e),
                ))
            }
        }
        let (server_reader, server_writer) = (socket.clone(), socket.clone());
        let (client_reader, client_writer) = tokio::io::split(inbound_stream);
        let (client_packet_reader, client_packet_writer) = (
            TrojanPacketReader::new(client_reader),
            TrojanPacketWriter::new(client_writer, request),
        );

        tokio::select!(
            _ = trojan::packet::packet_reader_to_udp_packet_writer(client_packet_reader, server_writer) => (),
            _ = trojan::packet::udp_packet_reader_to_packet_writer(server_reader, client_packet_writer) => ()
        );

        Ok((0, 0))
    }
}
