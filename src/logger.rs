struct Logger<T: core::fmt::Write> {
    pub tag: &'static str,
    pub output: T,
}

impl<T: core::fmt::Write> Logger<T> {
    #[allow(dead_code)]
    pub fn new(tag: &'static str, output: T) -> Self {
        Self { tag, output }
    }

    fn write_tag(&mut self) -> core::fmt::Result {
        self.output.write_str(self.tag)?;
        self.output.write_char(' ')
    }
}

impl<T: core::fmt::Write> core::fmt::Write for Logger<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_tag()?;
        self.output.write_str(s)?;
        self.output.write_char('\n')
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.write_tag()?;
        self.output.write_char(c)?;
        self.output.write_char('\n')
    }

    fn write_fmt(self: &mut Self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        self.write_tag()?;
        self.output.write_fmt(args)?;
        self.output.write_char('\n')
    }
}
