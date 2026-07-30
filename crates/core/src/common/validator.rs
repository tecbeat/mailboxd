//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use email_address::EmailAddress;

use poem_openapi::Validator;

pub struct EmailValidator;

impl Display for EmailValidator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Not a valid email address")
    }
}


impl Validator<String> for EmailValidator {
    fn check(&self, value: &String) -> bool {
        match EmailAddress::from_str(value) {
            Ok(e) => &e.email() == value,
            Err(_) => false,
        }
    }
}