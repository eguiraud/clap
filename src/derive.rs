//! This module contains traits that are usable with the `#[derive(...)].`
//! macros in [`clap_derive`].

use crate::builder::PossibleValue;
use crate::{ArgMatches, Command, Error};

use std::ffi::OsString;

/// Parse command-line arguments into `Self`.
///
/// The primary one-stop-shop trait used to create an instance of a `clap`
/// [`Command`], conduct the parsing, and turn the resulting [`ArgMatches`] back
/// into concrete instance of the user struct.
///
/// This trait is primarily a convenience on top of [`FromArgMatches`] +
/// [`CommandFactory`] which uses those two underlying traits to build the two
/// fundamental functions `parse` which uses the `std::env::args_os` iterator,
/// and `parse_from` which allows the consumer to supply the iterator (along
/// with fallible options for each).
///
/// See also [`Subcommand`] and [`Args`].
///
/// See the [derive reference](crate::_derive) for attributes and best practices.
///
/// **NOTE:** Deriving requires the [`derive` feature flag][crate::_features]
///
/// # Examples
///
/// The following example creates a `Context` struct that would be used
/// throughout the application representing the normalized values coming from
/// the CLI.
///
#[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
#[cfg_attr(feature = "derive", doc = " ```")]
/// /// My super CLI
/// #[derive(clap::Parser)]
/// #[command(name = "demo")]
/// struct Context {
///     /// More verbose output
///     #[arg(long)]
///     verbose: bool,
///     /// An optional name
///     #[arg(short, long)]
///     name: Option<String>,
/// }
/// ```
///
/// The equivalent [`Command`] struct + `From` implementation:
///
/// ```rust
/// # use clap::{Command, Arg, ArgMatches, ArgAction};
/// Command::new("demo")
///     .about("My super CLI")
///     .arg(Arg::new("verbose")
///         .long("verbose")
///         .action(ArgAction::SetTrue)
///         .help("More verbose output"))
///     .arg(Arg::new("name")
///         .long("name")
///         .short('n')
///         .help("An optional name")
///         .action(ArgAction::Set));
///
/// struct Context {
///     verbose: bool,
///     name: Option<String>,
/// }
///
/// impl From<ArgMatches> for Context {
///     fn from(m: ArgMatches) -> Self {
///         Context {
///             verbose: *m.get_one::<bool>("verbose").expect("defaulted_by_clap"),
///             name: m.get_one::<String>("name").cloned(),
///         }
///     }
/// }
/// ```
///
pub trait Parser: FromArgMatches + CommandFactory + Sized {
    /// Parse from `std::env::args_os()`, exit on error
    fn parse() -> Self {
        let mut matches = <Self as CommandFactory>::command().get_matches();
        let res = <Self as FromArgMatches>::from_arg_matches_mut(&mut matches)
            .map_err(format_error::<Self>);
        match res {
            Ok(s) => s,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches`
                e.exit()
            }
        }
    }

    /// Parse from `std::env::args_os()`, return Err on error.
    fn try_parse() -> Result<Self, Error> {
        let mut matches = ok!(<Self as CommandFactory>::command().try_get_matches());
        <Self as FromArgMatches>::from_arg_matches_mut(&mut matches).map_err(format_error::<Self>)
    }

    /// Parse from iterator, exit on error
    fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = <Self as CommandFactory>::command().get_matches_from(itr);
        let res = <Self as FromArgMatches>::from_arg_matches_mut(&mut matches)
            .map_err(format_error::<Self>);
        match res {
            Ok(s) => s,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches_from`
                e.exit()
            }
        }
    }

    /// Parse from iterator, return Err on error.
    fn try_parse_from<I, T>(itr: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = ok!(<Self as CommandFactory>::command().try_get_matches_from(itr));
        <Self as FromArgMatches>::from_arg_matches_mut(&mut matches).map_err(format_error::<Self>)
    }

    /// Update from iterator, exit on error
    fn update_from<I, T>(&mut self, itr: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = <Self as CommandFactory>::command_for_update().get_matches_from(itr);
        let res = <Self as FromArgMatches>::update_from_arg_matches_mut(self, &mut matches)
            .map_err(format_error::<Self>);
        if let Err(e) = res {
            // Since this is more of a development-time error, we aren't doing as fancy of a quit
            // as `get_matches_from`
            e.exit()
        }
    }

    /// Update from iterator, return Err on error.
    fn try_update_from<I, T>(&mut self, itr: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches =
            ok!(<Self as CommandFactory>::command_for_update().try_get_matches_from(itr));
        <Self as FromArgMatches>::update_from_arg_matches_mut(self, &mut matches)
            .map_err(format_error::<Self>)
    }
}

/// Create a [`Command`] relevant for a user-defined container.
///
/// Derived as part of [`Parser`].
pub trait CommandFactory: Sized {
    /// Build a [`Command`] that can instantiate `Self`.
    ///
    /// See [`FromArgMatches::from_arg_matches_mut`] for instantiating `Self`.
    fn command() -> Command;
    /// Build a [`Command`] that can update `self`.
    ///
    /// See [`FromArgMatches::update_from_arg_matches_mut`] for updating `self`.
    fn command_for_update() -> Command;
}

/// Converts an instance of [`ArgMatches`] to a user-defined container.
///
/// Derived as part of [`Parser`], [`Args`], and [`Subcommand`].
pub trait FromArgMatches: Sized {
    /// Instantiate `Self` from [`ArgMatches`], parsing the arguments as needed.
    ///
    /// Motivation: If our application had two CLI options, `--name
    /// <STRING>` and the flag `--debug`, we may create a struct as follows:
    ///
    #[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
    #[cfg_attr(feature = "derive", doc = " ```no_run")]
    /// struct Context {
    ///     name: String,
    ///     debug: bool
    /// }
    /// ```
    ///
    /// We then need to convert the `ArgMatches` that `clap` generated into our struct.
    /// `from_arg_matches` serves as the equivalent of:
    ///
    #[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
    #[cfg_attr(feature = "derive", doc = " ```no_run")]
    /// # use clap::ArgMatches;
    /// # struct Context {
    /// #   name: String,
    /// #   debug: bool
    /// # }
    /// impl From<ArgMatches> for Context {
    ///    fn from(m: ArgMatches) -> Self {
    ///        Context {
    ///            name: m.get_one::<String>("name").unwrap().clone(),
    ///            debug: *m.get_one::<bool>("debug").expect("defaulted by clap"),
    ///        }
    ///    }
    /// }
    /// ```
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error>;

    /// Instantiate `Self` from [`ArgMatches`], parsing the arguments as needed.
    ///
    /// Motivation: If our application had two CLI options, `--name
    /// <STRING>` and the flag `--debug`, we may create a struct as follows:
    ///
    #[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
    #[cfg_attr(feature = "derive", doc = " ```no_run")]
    /// struct Context {
    ///     name: String,
    ///     debug: bool
    /// }
    /// ```
    ///
    /// We then need to convert the `ArgMatches` that `clap` generated into our struct.
    /// `from_arg_matches_mut` serves as the equivalent of:
    ///
    #[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
    #[cfg_attr(feature = "derive", doc = " ```no_run")]
    /// # use clap::ArgMatches;
    /// # struct Context {
    /// #   name: String,
    /// #   debug: bool
    /// # }
    /// impl From<ArgMatches> for Context {
    ///    fn from(m: ArgMatches) -> Self {
    ///        Context {
    ///            name: m.get_one::<String>("name").unwrap().to_string(),
    ///            debug: *m.get_one::<bool>("debug").expect("defaulted by clap"),
    ///        }
    ///    }
    /// }
    /// ```
    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, Error> {
        Self::from_arg_matches(matches)
    }

    /// Assign values from `ArgMatches` to `self`.
    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error>;

    /// Assign values from `ArgMatches` to `self`.
    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), Error> {
        self.update_from_arg_matches(matches)
    }
}

/// Parse a set of arguments into a user-defined container.
///
/// Implementing this trait lets a parent container delegate argument parsing behavior to `Self`.
/// with:
/// - `#[command(flatten)] args: ChildArgs`: Attribute can only be used with struct fields that impl
///   `Args`.
/// - `Variant(ChildArgs)`: No attribute is used with enum variants that impl `Args`.
///
/// See the [derive reference](crate::_derive) for attributes and best practices.
///
/// **NOTE:** Deriving requires the [`derive` feature flag][crate::_features]
///
/// # Example
///
#[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
#[cfg_attr(feature = "derive", doc = " ```")]
/// #[derive(clap::Parser)]
/// struct Args {
///     #[command(flatten)]
///     logging: LogArgs,
/// }
///
/// #[derive(clap::Args)]
/// struct LogArgs {
///     #[arg(long, short = 'v', action = clap::ArgAction::Count)]
///     verbose: u8,
/// }
/// ```
pub trait Args: FromArgMatches + Sized {
    /// Report the [`ArgGroup::id`][crate::ArgGroup::id] for this set of arguments
    fn group_id() -> Option<crate::Id> {
        None
    }
    /// Append to [`Command`] so it can instantiate `Self`.
    ///
    /// See also [`CommandFactory`].
    fn augment_args(cmd: Command) -> Command;
    /// Append to [`Command`] so it can update `self`.
    ///
    /// This is used to implement `#[command(flatten)]`
    ///
    /// See also [`CommandFactory`].
    fn augment_args_for_update(cmd: Command) -> Command;
}

/// Parse a sub-command into a user-defined enum.
///
/// Implementing this trait lets a parent container delegate subcommand behavior to `Self`.
/// with:
/// - `#[command(subcommand)] field: SubCmd`: Attribute can be used with either struct fields or enum
///   variants that impl `Subcommand`.
/// - `#[command(flatten)] Variant(SubCmd)`: Attribute can only be used with enum variants that impl
///   `Subcommand`.
///
/// See the [derive reference](crate::_derive) for attributes and best practices.
///
/// **NOTE:** Deriving requires the [`derive` feature flag][crate::_features]
///
/// # Example
///
#[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
#[cfg_attr(feature = "derive", doc = " ```")]
/// #[derive(clap::Parser)]
/// struct Args {
///     #[command(subcommand)]
///     action: Action,
/// }
///
/// #[derive(clap::Subcommand)]
/// enum Action {
///     Add,
///     Remove,
/// }
/// ```
pub trait Subcommand: FromArgMatches + Sized {
    /// Append to [`Command`] so it can instantiate `Self`.
    ///
    /// See also [`CommandFactory`].
    fn augment_subcommands(cmd: Command) -> Command;
    /// Append to [`Command`] so it can update `self`.
    ///
    /// This is used to implement `#[command(flatten)]`
    ///
    /// See also [`CommandFactory`].
    fn augment_subcommands_for_update(cmd: Command) -> Command;
    /// Test whether `Self` can parse a specific subcommand
    fn has_subcommand(name: &str) -> bool;
}

/// Parse arguments into enums.
///
/// When deriving [`Parser`], a field whose type implements `ValueEnum` can have the attribute
/// `#[arg(value_enum)]` which will
/// - Call [`EnumValueParser`][crate::builder::EnumValueParser]
/// - Allowing using the `#[arg(default_value_t)]` attribute without implementing `Display`.
///
/// See the [derive reference](crate::_derive) for attributes and best practices.
///
/// **NOTE:** Deriving requires the [`derive` feature flag][crate::_features]
///
/// # Example
///
#[cfg_attr(not(feature = "derive"), doc = " ```ignore")]
#[cfg_attr(feature = "derive", doc = " ```")]
/// #[derive(clap::Parser)]
/// struct Args {
///     #[arg(value_enum)]
///     level: Level,
/// }
///
/// #[derive(clap::ValueEnum, Clone)]
/// enum Level {
///     Debug,
///     Info,
///     Warning,
///     Error,
/// }
/// ```
pub trait ValueEnum: Sized + Clone {
    /// All possible argument values, in display order.
    fn value_variants<'a>() -> &'a [Self];

    /// Parse an argument into `Self`.
    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        Self::value_variants()
            .iter()
            .find(|v| {
                v.to_possible_value()
                    .expect("ValueEnum::value_variants contains only values with a corresponding ValueEnum::to_possible_value")
                    .matches(input, ignore_case)
            })
            .cloned()
            .ok_or_else(|| format!("invalid variant: {}", input))
    }

    /// The canonical argument value.
    ///
    /// The value is `None` for skipped variants.
    fn to_possible_value(&self) -> Option<PossibleValue>;
}

impl<T: Parser> Parser for Box<T> {
    fn parse() -> Self {
        Box::new(<T as Parser>::parse())
    }

    fn try_parse() -> Result<Self, Error> {
        <T as Parser>::try_parse().map(Box::new)
    }

    fn parse_from<I, It>(itr: I) -> Self
    where
        I: IntoIterator<Item = It>,
        It: Into<OsString> + Clone,
    {
        Box::new(<T as Parser>::parse_from(itr))
    }

    fn try_parse_from<I, It>(itr: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = It>,
        It: Into<OsString> + Clone,
    {
        <T as Parser>::try_parse_from(itr).map(Box::new)
    }
}

impl<T: CommandFactory> CommandFactory for Box<T> {
    fn command<'help>() -> Command {
        <T as CommandFactory>::command()
    }
    fn command_for_update<'help>() -> Command {
        <T as CommandFactory>::command_for_update()
    }
}

impl<T: FromArgMatches> FromArgMatches for Box<T> {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        <T as FromArgMatches>::from_arg_matches(matches).map(Box::new)
    }
    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, Error> {
        <T as FromArgMatches>::from_arg_matches_mut(matches).map(Box::new)
    }
    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        <T as FromArgMatches>::update_from_arg_matches(self, matches)
    }
    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), Error> {
        <T as FromArgMatches>::update_from_arg_matches_mut(self, matches)
    }
}

impl<T: Args> Args for Box<T> {
    fn augment_args(cmd: Command) -> Command {
        <T as Args>::augment_args(cmd)
    }
    fn augment_args_for_update(cmd: Command) -> Command {
        <T as Args>::augment_args_for_update(cmd)
    }
}

impl<T: Subcommand> Subcommand for Box<T> {
    fn augment_subcommands(cmd: Command) -> Command {
        <T as Subcommand>::augment_subcommands(cmd)
    }
    fn augment_subcommands_for_update(cmd: Command) -> Command {
        <T as Subcommand>::augment_subcommands_for_update(cmd)
    }
    fn has_subcommand(name: &str) -> bool {
        <T as Subcommand>::has_subcommand(name)
    }
}

fn format_error<I: CommandFactory>(err: crate::Error) -> crate::Error {
    let mut cmd = I::command();
    err.format(&mut cmd)
}
