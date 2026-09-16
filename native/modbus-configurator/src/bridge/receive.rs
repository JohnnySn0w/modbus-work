//! Receive deadlines and metadata-only progress on the existing transport.
use super::*;
impl BridgeSession {
    // A recognized prompt must be followed by quiet input. This allows later
    // menu fields/identity/summary to arrive in separate serial reads.
    pub(super) fn receive(
        &mut self,
        cancel: &AtomicBool,
        deadline: Instant,
        allow_silent: bool,
    ) -> Result<()> {
        self.receive_segment(cancel, deadline, allow_silent, false)
    }

    pub(super) fn receive_segment(
        &mut self,
        cancel: &AtomicBool,
        deadline: Instant,
        allow_silent: bool,
        preserve: bool,
    ) -> Result<()> {
        self.received_bytes = 0;
        let mut last_data = Instant::now();
        let mut last_recovery = Instant::now();
        let mut recoveries = 0;
        let mut received = false;
        let mut total = if preserve {
            self.parser.text().len()
        } else {
            0
        };
        let mut bytes = [0; 1024];
        let started = Instant::now();
        let mut last_notice = started;
        self.trace_event(&format!(
            "Receive started: deadline in {} ms; previous parser state {:?}",
            deadline.saturating_duration_since(started).as_millis(),
            self.parser.state()
        ));
        loop {
            if let Err(mut error) = Self::check(cancel, deadline) {
                let detail = format!(
                    "elapsed {} ms; received {} bytes; last data {} ms ago; parser state {:?}; receive recovery attempts {recoveries}",
                    started.elapsed().as_millis(),
                    self.received_bytes,
                    last_data.elapsed().as_millis(),
                    self.parser.state()
                );
                self.trace_event(&format!("Receive ended: {:?}; {detail}", error.code));
                error.message = format!(
                    "{} {detail}. {}",
                    error.message,
                    if received {
                        "A response arrived but did not reach a recognized complete prompt."
                    } else {
                        "No response bytes arrived during this wait."
                    }
                );
                return Err(error);
            }
            if last_notice.elapsed() >= Duration::from_secs(5) {
                self.trace_event(&format!("Waiting: elapsed {} ms; received {} bytes; last data {} ms ago; parser state {:?}", started.elapsed().as_millis(), self.received_bytes, last_data.elapsed().as_millis(), self.parser.state()));
                last_notice = Instant::now();
            }
            match self.transport.read(&mut bytes) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
                Ok(count) => {
                    if !received {
                        self.trace_event(&format!(
                            "First response bytes after {} ms",
                            started.elapsed().as_millis()
                        ));
                    }
                    self.received_bytes += count;
                    if !received && !preserve {
                        self.parser.reset();
                    }
                    received = true;
                    total += count;
                    if total > crate::console::MAX_RESPONSE_BYTES {
                        return Err(BridgeError::new(
                            ErrorCode::InvalidResponse,
                            "Console response exceeded the supported size.",
                        ));
                    }
                    self.parser.feed(&bytes[..count]);
                    last_data = Instant::now();
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    if self.transport.continuous_receive()
                        && (!allow_silent || received)
                        && !self.response_ready()
                        && last_data.elapsed() >= Duration::from_millis(500)
                        && last_recovery.elapsed() >= Duration::from_millis(500)
                        && recoveries < 16
                    {
                        recoveries += 1;
                        match self.transport.resume_receive() {
                            Ok(()) => self.trace_event(&format!("Receive recovery attempt {recoveries} returned successfully; receipt of further bytes is not confirmed; no command resent")),
                            Err(e) if e.kind() == io::ErrorKind::Unsupported => self.trace_event("Receive recovery unsupported by this transport"),
                            Err(e) => {
                                self.trace_event(&format!("Receive recovery failed: {:?}; operating system code {:?}", e.kind(), e.raw_os_error()));
                                return Err(e.into());
                            }
                        }
                        last_recovery = Instant::now();
                    }
                    if !self.transport.continuous_receive()
                        && received
                        && last_data.elapsed() >= self.timing.quiet
                        && !self.response_ready()
                    {
                        return Err(BridgeError::new(
                            ErrorCode::Timeout,
                            "Console reply stalled before its final prompt.",
                        ));
                    }
                    if last_data.elapsed() >= self.timing.quiet
                        && ((received && self.response_ready()) || (allow_silent && !received))
                    {
                        self.trace_event(&format!(
                            "Receive completed: {} ms; {} bytes; parser state {:?}",
                            started.elapsed().as_millis(),
                            self.received_bytes,
                            self.parser.state()
                        ));
                        return Ok(());
                    }
                }
                Err(e) => {
                    self.trace_event(&format!(
                        "Receive failed after {} ms and {} bytes: {:?}; operating system code {:?}",
                        started.elapsed().as_millis(),
                        self.received_bytes,
                        e.kind(),
                        e.raw_os_error()
                    ));
                    return Err(e.into());
                }
            }
        }
    }
}
