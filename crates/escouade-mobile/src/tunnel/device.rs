//! The interface smoltcp sees: IP packets in two queues, filled and emptied by the WireGuard side of the engine.

use std::collections::VecDeque;

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

/// The tunnel's MTU: WireGuard's usual 1420 (1500, less its 80 bytes over IPv6).
pub const MTU: usize = 1420;

#[derive(Default)]
pub struct Queues {
    /// Packets out of the tunnel, for smoltcp.
    pub rx: VecDeque<Vec<u8>>,
    /// Packets from smoltcp, for the tunnel.
    pub tx: VecDeque<Vec<u8>>,
}

pub struct Rx(Vec<u8>);
pub struct Tx<'a>(&'a mut VecDeque<Vec<u8>>);

impl RxToken for Rx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.0)
    }
}

impl TxToken for Tx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut packet = vec![0u8; len];
        let r = f(&mut packet);
        self.0.push_back(packet);
        r
    }
}

impl Device for Queues {
    type RxToken<'a> = Rx;
    type TxToken<'a> = Tx<'a>;

    fn receive(&mut self, _: Instant) -> Option<(Rx, Tx<'_>)> {
        let packet = self.rx.pop_front()?;
        Some((Rx(packet), Tx(&mut self.tx)))
    }

    fn transmit(&mut self, _: Instant) -> Option<Tx<'_>> {
        Some(Tx(&mut self.tx))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        caps
    }
}
