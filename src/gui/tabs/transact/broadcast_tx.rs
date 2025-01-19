use flume::Sender;
use redgold_schema::structs::Transaction;
use redgold_schema::util::lang_util::JsonCombineResult;
use redgold_common::flume_send_help::SendErrorInfo;
use crate::gui::app_loop::LocalState;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use crate::node_config::ApiNodeConfig;
