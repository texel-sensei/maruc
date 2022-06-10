//! Implementation of a simple string wrapper for secrets.
//! Inspired by the secrecy crate (which we can't use, because of version conflicts with the
//! matrix-sdk

// Code taken is under the MIT license
// MIT License
//
// Copyright (c) 2019 iqlusion
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::fmt::{self, Debug};
use zeroize::Zeroize;

pub struct Secret<S: Zeroize> {
    inner_secret: S,
}

pub type SecretString = Secret<String>;

pub trait ExposeSecret<S> {
    fn expose_secret(&self) -> &S;
}

impl<S> Secret<S>
where
    S: Zeroize,
{
    /// Take ownership of a secret value
    pub fn new(secret: S) -> Self {
        Secret {
            inner_secret: secret,
        }
    }
}

impl<S> ExposeSecret<S> for Secret<S>
where
    S: Zeroize,
{
    fn expose_secret(&self) -> &S {
        &self.inner_secret
    }
}

impl<S> From<S> for Secret<S>
where
    S: Zeroize,
{
    fn from(secret: S) -> Self {
        Self::new(secret)
    }
}

impl<S> Clone for Secret<S>
where
    S: Clone + Zeroize,
{
    fn clone(&self) -> Self {
        Secret {
            inner_secret: self.inner_secret.clone(),
        }
    }
}

impl<S> Debug for Secret<S>
where
    S: Zeroize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret([REDACTED] {})", std::any::type_name::<S>())
    }
}

impl<S> Drop for Secret<S>
where
    S: Zeroize,
{
    fn drop(&mut self) {
        // Zero the secret out from memory
        self.inner_secret.zeroize();
    }
}
