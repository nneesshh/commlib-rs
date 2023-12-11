//!
//! Common Library: logger
//!

type SinkVec = Vec<std::sync::Arc<dyn spdlog::sink::Sink>>;

pub use spdlog::Level as LogLevel;

/// Initialize logger
pub fn init(path: &std::path::PathBuf, name: &str, level: u16, log_to_console: bool) {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        // run initialization here
        init_logger_once(path, name, level, log_to_console);
    });
}

fn init_logger_once(path: &std::path::PathBuf, name: &str, level: u16, log_to_console: bool) {
    let log_path: std::path::PathBuf = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("log")
        .join(path.as_os_str());

    let log_name_pattern = format!("{}.log", name);
    let log_path_with_pattern = log_path.join(log_name_pattern);

    // Sink: file_sink
    let file_sink_result = spdlog::sink::DateAndHourRotatingFileSink::builder()
        .base_path(log_path_with_pattern.clone())
        .rotate_on_open(false)
        .build();
    let file_sink = match file_sink_result {
        Ok(sink) => std::sync::Arc::new(sink),
        Err(err) => {
            println!(
                "log_path_with_pattern={:?}, create file sink failed!!! error: {:?}",
                log_path_with_pattern, err
            );
            std::panic!()
        }
    };

    // Sink: ss_sink
    let ss_sink = std::sync::Arc::new(
        spdlog::sink::StdStreamSink::builder()
            .std_stream(spdlog::sink::StdStream::Stdout)
            .style_mode(spdlog::terminal_style::StyleMode::Never)
            .build()
            .unwrap(),
    );

    let mut sinks = SinkVec::new();
    sinks.push(file_sink);

    if log_to_console {
        sinks.push(ss_sink);
    }

    {
        // Building a `AsyncPoolSink`.
        // Log and flush operations with this sink will be processed asynchronously.
        let async_sink = std::sync::Arc::new(
            spdlog::sink::AsyncPoolSink::builder()
                .sinks(sinks)
                .build()
                .unwrap(),
        );

        let logger: std::sync::Arc<spdlog::Logger> =
            std::sync::Arc::new(spdlog::Logger::builder().sink(async_sink).build().unwrap());

        // Log level filter
        let mut log_level = spdlog::Level::Info;
        if spdlog::Level::Critical as u16 <= level && level < spdlog::Level::Trace as u16 {
            log_level = spdlog::Level::from_usize(level as usize).unwrap();
        }
        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(log_level));

        // Flush when error
        let flush_level = spdlog::Level::Error;
        logger.set_flush_level_filter(spdlog::LevelFilter::MoreSevereEqual(flush_level));

        println!(
            "[init_logger_once()]: log_path_with_pattern={:?}, name={}, level={:?}, flush_level={:?}",
            log_path_with_pattern, name, log_level, flush_level
        );

        #[cfg(windows)]
        // From now on, auto-flush the `logger` buffer every 1 seconds.
        logger.set_flush_period(Some(std::time::Duration::from_secs(1)));

        #[cfg(unix)]
        // From now on, auto-flush the `logger` buffer every 3 seconds.
        logger.set_flush_period(Some(std::time::Duration::from_secs(3)));

        // Notice: "logger" is moved here
        spdlog::set_default_logger(logger);
    }

    let log_dir = match std::fs::read_dir(&log_path) {
        Ok(dir) => dir,
        Err(err) => {
            println!("log_path: {:?}, error: {:?}!!!", log_path, err);
            std::panic!();
        }
    };

    let files_in_log_path = log_dir
        .collect::<Vec<std::io::Result<std::fs::DirEntry>>>()
        .into_iter()
        .map(|p| p.unwrap().file_name())
        .collect::<Vec<std::ffi::OsString>>();

    let log_level = spdlog::default_logger().level_filter();
    let flush_level = spdlog::default_logger().flush_level_filter();
    spdlog::info!("log_path_with_pattern: {:?}, name: {}, level: {:?}, flush_level: {:?}, all log files: {:?} => {:?}",
        log_path_with_pattern, name, log_level, flush_level, log_path, files_in_log_path);
    spdlog::default_logger().flush();

    // Building a custom formatter.
    let new_formatter: Box<spdlog::formatter::CommlibFormatter> = Box::default();

    // Setting the new formatter for each sink of the default logger.
    for sink in spdlog::default_logger().sinks() {
        sink.set_formatter(new_formatter.clone())
    }

    // Bind default logger to proxy
    let proxy: &'static spdlog::LogCrateProxy = spdlog::log_crate_proxy();
    proxy.set_logger(Some(spdlog::default_logger()));

    // Call this function early. Logs from log crate will not be handled before
    // calling it.
    spdlog::init_log_crate_proxy()
        .expect("users should only call `init_log_crate_proxy` function once");

    log::set_max_level(log::LevelFilter::Trace);
    log::info!("this is a log from other crate");
}

#[cfg(test)]
mod tests {

    use crate::init;
    use crate::LogLevel;

    #[test]
    fn format() {
        let log_path = std::path::PathBuf::from("log");
        let log_level = LogLevel::Info as u16;
        init(&log_path, "auto-dragon", log_level, true);
    }
}
