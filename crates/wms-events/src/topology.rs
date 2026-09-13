use lapin::options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::{AMQPValue, FieldTable};
use lapin::{Channel, ExchangeKind};

pub const EXCHANGE: &str = "wms.events";
pub const DEAD_LETTER_EXCHANGE: &str = "wms.events.dlx";

pub const ROUTING_STOCK_RECEIVED: &str = "inventory.stock.received";

pub const NOTIFICATION_QUEUE: &str = "notification.stock-events";
pub const NOTIFICATION_DLQ: &str = "notification.stock-events.dlq";

// A SECOND consumer of the very same events. On a topic exchange each queue gets
// its own copy, so adding this one takes nothing away from notification-service.
// That is the difference between pub/sub and a work queue: here consumers are
// independent subscribers, not competitors for the same messages.
pub const PRODUCT_QUEUE: &str = "product.stock-events";
pub const PRODUCT_DLQ: &str = "product.stock-events.dlq";

pub const STOCK_BINDING: &str = "inventory.stock.#";

// Declaring is idempotent in AMQP, so every process can safely declare what it
// needs at startup rather than relying on someone having set the broker up by hand.
pub async fn declare(channel: &Channel) -> anyhow::Result<()> {
    channel
        .exchange_declare(
            EXCHANGE.into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..ExchangeDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .exchange_declare(
            DEAD_LETTER_EXCHANGE.into(),
            ExchangeKind::Fanout,
            ExchangeDeclareOptions {
                durable: true,
                ..ExchangeDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    // The DLQ is set up on day one, not after the first incident. A message that
    // fails deterministically has to go somewhere it can be inspected, otherwise
    // the only options are losing it or looping on it forever.
    declare_subscriber(channel, NOTIFICATION_QUEUE, NOTIFICATION_DLQ).await?;
    declare_subscriber(channel, PRODUCT_QUEUE, PRODUCT_DLQ).await?;

    tracing::info!(exchange = EXCHANGE, "event topology declared");
    Ok(())
}

// One durable queue bound to the stock events, plus its own dead letter queue.
async fn declare_subscriber(channel: &Channel, queue: &str, dlq: &str) -> anyhow::Result<()> {
    channel
        .queue_declare(
            dlq.into(),
            QueueDeclareOptions::durable(),
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            dlq.into(),
            DEAD_LETTER_EXCHANGE.into(),
            "".into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut args = FieldTable::default();
    args.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString(DEAD_LETTER_EXCHANGE.into()),
    );

    channel
        .queue_declare(queue.into(), QueueDeclareOptions::durable(), args)
        .await?;

    channel
        .queue_bind(
            queue.into(),
            EXCHANGE.into(),
            STOCK_BINDING.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}
