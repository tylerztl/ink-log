// Copyright Patract Labs Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Ink! logger that prints all messages with a readable output format.
pub use ink_prelude::{
    format,
    string::String,
    vec::Vec,
};
pub use log::{
    Level,
    LevelFilter,
};
use scale::{
    Decode,
    Encode,
};

/// Ink! contract logger that supports on-chain and off-chain print.
pub struct InkLogger {
    /// The default logging level
    default_level: LevelFilter,
    /// The specific logging level for each module
    module_levels: Vec<(String, LevelFilter)>,
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct LogRecord {
    pub level: u32,
    pub target: Vec<u8>,
    pub args: Vec<u8>,
}

// syntactic sugar for logging.
#[macro_export]
macro_rules! clog {
    ($level:tt, target: $target:expr, $patter:expr $(, $values:expr)* $(,)?) => (
        ink_log::InkLogger::new().init();
        ink_log::$level!(
            target: $target,
		    $patter $(, $values)*
		);
    );
    ($level:tt, $patter:expr $(, $values:expr)* $(,)?) => (
        ink_log::InkLogger::new().init();
        ink_log::$level!(
		    $patter $(, $values)*
		);
    )
}

// func_id refer to https://github.com/patractlabs/PIPs/blob/main/PIPs/pip-100.md
// 0xfeffff00
pub const FUNC_ID_LOG: u32 = 0xfeffff00;

impl InkLogger {
    /// Initializes the global logger with a InkLogger instance with
    /// default log level set to `Level::Trace`.
    pub fn new() -> InkLogger {
        InkLogger {
            default_level: LevelFilter::Trace,
            module_levels: Vec::new(),
        }
    }

    /// Initialize the logger when running natively (`std`)
    #[cfg(feature = "std")]
    pub fn init(mut self) {
        // Sort all module levels from most specific to least specific. The length of the module
        // name is used instead of its actual depth to avoid module name parsing.
        self.module_levels
            .sort_by_key(|(name, _level)| name.len().wrapping_neg());
        let max_level = self
            .module_levels
            .iter()
            .map(|(_name, level)| level)
            .copied()
            .max();
        let max_level = max_level
            .map(|lvl| lvl.max(self.default_level))
            .unwrap_or(self.default_level);
        log::set_max_level(max_level);
        let _ = log::set_boxed_logger(Box::new(self));
    }

    /// Initialize the logger when running webAssembly (`no_std`)
    ///
    /// This function may only be called once.
    #[cfg(not(feature = "std"))]
    pub fn init(mut self) {
        // Sort all module levels from most specific to least specific. The length of the module
        // name is used instead of its actual depth to avoid module name parsing.
        self.module_levels
            .sort_by_key(|(name, _level)| name.len().wrapping_neg());
        let max_level = self
            .module_levels
            .iter()
            .map(|(_name, level)| level)
            .copied()
            .max();
        let max_level = max_level
            .map(|lvl| lvl.max(self.default_level))
            .unwrap_or(self.default_level);
        log::set_max_level(max_level);
        static LOGGER: InkLogger = InkLogger {
            default_level: LevelFilter::Trace,
            module_levels: Vec::new(),
        };
        let _ = log::set_logger(&LOGGER);
    }

    /// Overrides the 'default' log level.
    ///
    /// ```no_run
    /// ink_env::InkLogger::new()
    ///     .with_level(ink_env::LevelFilter::Info)
    ///     .init();
    ///
    /// ink_env::info!("cool ink! logger!");
    /// ```
    pub fn with_level(mut self, level: LevelFilter) -> InkLogger {
        self.default_level = level;
        self
    }

    /// Overrides the log level for some specific modules.
    ///
    /// ```no_run
    /// ink_env::InkLogger::new()
    ///     .with_module_level("ink_contract", ink_env::LevelFilter::Debug)
    ///     .init();
    ///
    /// ink_env::debug!(target: "ink_contract", "cool ink! logger!");
    /// ```
    pub fn with_module_level(mut self, target: &str, level: LevelFilter) -> InkLogger {
        self.module_levels.push((String::from(target), level));
        self
    }
}

impl Default for InkLogger {
    fn default() -> Self {
        InkLogger::new()
    }
}

impl log::Log for InkLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        &metadata.level().to_level_filter()
            <= self
                .module_levels
                .iter()
                .find(|(name, _level)| metadata.target().starts_with(name))
                .map(|(_name, level)| level)
                .unwrap_or(&self.default_level)
    }

    fn log(&self, record: &log::Record) {
        let input = LogRecord {
            level: record.level() as u32,
            target: Vec::from(record.target()),
            args: Vec::from(ink_prelude::format!("{}", record.args())),
        };
        ink_env::call_chain_extension::<LogRecord, LogRecord>(FUNC_ID_LOG, &input)
            .unwrap();
    }

    fn flush(&self) {}
}