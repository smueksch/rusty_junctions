//! Various channel implementations for different purposes.
//!
//! All of the channels within this module can be used to build up Join Patterns.
//! However, they serve different functions within the patterns. For instance,
//! a `RecvChannel` is used to get the value generated by a Join Pattern firing
//! asynchronously.

use std::marker::PhantomData;
use std::sync::mpsc::{channel, RecvError, SendError, Sender};
use std::{any::Any, marker::Send};

use super::types::{ids, Message, Packet};

/***************************
 * Sending Channel Structs *
 ***************************/

/// Asynchronous, message sending channel.
///
/// This channel type is characterized by the argument type of its `send` method.
/// It will only be able to send messages to the Junction but not recover values
/// generated by Join Patterns that have been fired.
///
/// Sending a message this channel will *not* block the current thread, but may
/// allow a Join Pattern that it is part of to fire.
#[derive(Clone)]
pub struct SendChannel<T> {
    id: ids::ChannelId,
    junction_id: ids::JunctionId,
    sender: Sender<Packet>,
    send_type: PhantomData<T>,
}

impl<T> SendChannel<T> {
    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }

    /// Return the ID of the `Junction` this channel is associated to.
    pub(crate) fn junction_id(&self) -> ids::JunctionId {
        self.junction_id
    }

    /// Create a stripped down representation of this channel.
    pub(crate) fn strip(&self) -> StrippedSendChannel<T> {
        StrippedSendChannel::new(self.id)
    }
}

impl<T> SendChannel<T>
where
    T: Any + Send,
{
    pub(crate) fn new(
        id: ids::ChannelId,
        junction_id: ids::JunctionId,
        sender: Sender<Packet>,
    ) -> SendChannel<T> {
        SendChannel {
            id,
            junction_id,
            sender,
            send_type: PhantomData,
        }
    }

    pub fn send(&self, value: T) -> Result<(), SendError<Packet>> {
        self.sender.send(Packet::Message {
            channel_id: self.id,
            msg: Message::new(value),
        })
    }
}

/// Stripped down version of `SendChannel`.
///
/// The main purpose of this struct is to be used in the Join Pattern types to
/// increase readability and maintainability.
///
/// This version of the `SendChannel` does not carry the same functionality as
/// the actual `SendChannel`, however, it holds the bare minimum information
/// necessary for the creation of Join Patterns. Specifically, this channel
/// cannot send and does not know of its Junction, but is able to provide a
/// channel ID and type associated with it.
pub(crate) struct StrippedSendChannel<T> {
    id: ids::ChannelId,
    send_type: PhantomData<T>,
}

impl<T> StrippedSendChannel<T> {
    pub(crate) fn new(id: ids::ChannelId) -> StrippedSendChannel<T> {
        StrippedSendChannel {
            id,
            send_type: PhantomData,
        }
    }

    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }
}

/*****************************
 * Receiving Channel Structs *
 *****************************/

/// Synchronous, value receiving channel.
///
/// This channel type is characterized by the return type of its `recv` method.
/// No messages can be sent through this channel, but the value generated by
/// running a Join Pattern
///
/// Sending a message on this channel *will* block the current thread until a Join
/// Pattern that this channel is part of has fired.
#[derive(Clone)]
pub struct RecvChannel<R> {
    id: ids::ChannelId,
    junction_id: ids::JunctionId,
    sender: Sender<Packet>,
    recv_type: PhantomData<R>,
}

impl<R> RecvChannel<R> {
    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }

    /// Return the ID of the `Junction` this channel is associated to.
    pub(crate) fn junction_id(&self) -> ids::JunctionId {
        self.junction_id
    }

    /// Create a stripped down representation of this channel.
    pub(crate) fn strip(&self) -> StrippedRecvChannel<R> {
        StrippedRecvChannel::new(self.id)
    }
}

impl<R> RecvChannel<R>
where
    R: Any + Send,
{
    pub(crate) fn new(
        id: ids::ChannelId,
        junction_id: ids::JunctionId,
        sender: Sender<Packet>,
    ) -> RecvChannel<R> {
        RecvChannel {
            id,
            junction_id,
            sender,
            recv_type: PhantomData,
        }
    }

    /// Receive value generated by fired Join Pattern.
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to send a return `Sender` to the Junction.
    pub fn recv(&self) -> Result<R, RecvError> {
        let (tx, rx) = channel::<R>();

        self.sender
            .send(Packet::Message {
                channel_id: self.id,
                msg: Message::new(tx),
            })
            .unwrap();

        rx.recv()
    }
}

/// Stripped down version of `RecvChannel`.
///
/// The main purpose of this struct is to be used in the Join Pattern types to
/// increase readability and maintainability.
///
/// This version of the `RecvChannel` does not carry the same functionality as
/// the actual `RecvChannel`, however, it holds the bare minimum information
/// necessary for the creation of Join Patterns. Specifically, this channel
/// cannot receive and does not know of its Junction, but is able to provide a
/// channel ID and type associated with it.
pub(crate) struct StrippedRecvChannel<R> {
    id: ids::ChannelId,
    recv_type: PhantomData<R>,
}

impl<R> StrippedRecvChannel<R> {
    pub(crate) fn new(id: ids::ChannelId) -> StrippedRecvChannel<R> {
        StrippedRecvChannel {
            id,
            recv_type: PhantomData,
        }
    }

    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }
}

/*********************************
 * Bidirectional Channel Structs *
 *********************************/

/// Synchronous, bidirectional message channel.
///
/// This channel type is characterized by both the argument and return type of its
/// `send_recv` method. A message can be sent through this channel which will then
/// also cause the channel to wait for a Join Pattern involving this channel to fire.
///
/// The subtle difference between using this channel type over a combination of a
/// `SendChannel` and a `RecvChannel` is that this channel ensures that `Message`s
/// necessary to perform the sending and receiving happen *atomically* together. In
/// fact, only one `Message` is sent for both. Therefore, a call to `send_recv`
/// can be viewed as an atomic operation, whereas a separate `SendChannel::send`
/// and `RecvChannel::recv` may have an arbitrary amount of actions happen between
/// them.
///
/// Sending a message on this channel *will* block the current thread until a Join
/// Pattern that this channel is part of has fired.
#[derive(Clone)]
pub struct BidirChannel<T, R> {
    id: ids::ChannelId,
    junction_id: ids::JunctionId,
    sender: Sender<Packet>,
    send_type: PhantomData<T>,
    recv_type: PhantomData<R>,
}

impl<T, R> BidirChannel<T, R> {
    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }

    /// Return the ID of the `Junction` this channel is associated to.
    pub(crate) fn junction_id(&self) -> ids::JunctionId {
        self.junction_id
    }

    /// Create a stripped down representation of this channel.
    pub(crate) fn strip(&self) -> StrippedBidirChannel<T, R> {
        StrippedBidirChannel::new(self.id)
    }
}

impl<T, R> BidirChannel<T, R>
where
    T: Any + Send,
    R: Any + Send,
{
    pub(crate) fn new(
        id: ids::ChannelId,
        junction_id: ids::JunctionId,
        sender: Sender<Packet>,
    ) -> BidirChannel<T, R> {
        BidirChannel {
            id,
            junction_id,
            sender,
            send_type: PhantomData,
            recv_type: PhantomData,
        }
    }

    /// Send a message and receive value generated by fired Junction.
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to send the given message and return
    /// `Sender` to the Junction.
    pub fn send_recv(&self, msg: T) -> Result<R, RecvError> {
        let (tx, rx) = channel::<R>();

        self.sender
            .send(Packet::Message {
                channel_id: self.id,
                msg: Message::new((msg, tx)),
            })
            .unwrap();

        rx.recv()
    }
}

/// Stripped down version of `BidirChannel`.
///
/// The main purpose of this struct is to be used in the Join Pattern types to
/// increase readability and maintainability.
///
/// This version of the `BidirChannel` does not carry the same functionality as
/// the actual `BidirChannel`, however, it holds the bare minimum information
/// necessary for the creation of Join Patterns. Specifically, this channel
/// cannot send or receive and does not know of its Junction, but is able to
/// provide a channel ID and type associated with it.
pub(crate) struct StrippedBidirChannel<T, R> {
    id: ids::ChannelId,
    send_type: PhantomData<T>,
    recv_type: PhantomData<R>,
}

impl<T, R> StrippedBidirChannel<T, R> {
    pub(crate) fn new(id: ids::ChannelId) -> StrippedBidirChannel<T, R> {
        StrippedBidirChannel {
            id,
            send_type: PhantomData,
            recv_type: PhantomData,
        }
    }

    /// Return the channel's ID.
    pub(crate) fn id(&self) -> ids::ChannelId {
        self.id
    }
}