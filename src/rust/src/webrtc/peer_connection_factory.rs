//
// Copyright (C) 2020 Signal Messenger, LLC.
// All rights reserved.
//
// SPDX-License-Identifier: GPL-3.0-only
//

//! WebRTC Peer Connection Interface
use std::fmt;

use crate::common::Result;
use crate::core::platform::Platform;
use crate::core::util::CppObject;
use crate::error::RingRtcError;
#[cfg(feature = "simnet")]
use crate::webrtc::injectable_network::InjectableNetwork;
use crate::webrtc::media::{AudioTrack, VideoSource};
use crate::webrtc::peer_connection::PeerConnection;
use crate::webrtc::peer_connection_observer::PeerConnectionObserver;
use std::ffi::CString;
use std::os::raw::c_char;

#[cfg(not(feature = "sim"))]
use crate::webrtc::ffi::peer_connection_factory as pcf;
#[cfg(not(feature = "sim"))]
use crate::webrtc::ffi::ref_count;

// TODO: sim::pcf
#[cfg(feature = "sim")]
use crate::webrtc::sim::peer_connection_factory as pcf;
#[cfg(feature = "sim")]
use crate::webrtc::sim::ref_count;

const ADM_MAX_DEVICE_NAME_SIZE: usize = 128;
const ADM_MAX_DEVICE_UUID_SIZE: usize = 128;

/// Rust wrapper around WebRTC C++ RTCCertificate object.
pub struct Certificate {
    rffi: *const pcf::RffiCertificate,
}

impl Certificate {
    pub fn generate() -> Result<Certificate> {
        let rffi = unsafe { pcf::Rust_generateCertificate() };
        if rffi.is_null() {
            return Err(RingRtcError::GenerateCertificate.into());
        }
        Ok(Self { rffi })
    }

    pub fn rffi(&self) -> *const pcf::RffiCertificate {
        self.rffi
    }
}

impl fmt::Display for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Certificate: {:p}", self.rffi)
    }
}

impl fmt::Debug for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Drop for Certificate {
    fn drop(&mut self) {
        debug!("Certificate::drop()");
        if !self.rffi.is_null() {
            ref_count::release_ref(self.rffi as CppObject);
        }
    }
}

impl Clone for Certificate {
    fn clone(&self) -> Self {
        debug!("Certificate::clone() {}", self.rffi as u64);
        if !self.rffi.is_null() {
            ref_count::add_ref(self.rffi as CppObject);
        }
        Self { rffi: self.rffi }
    }
}

unsafe impl Send for Certificate {}
unsafe impl Sync for Certificate {}

#[repr(C)]
pub struct RffiIceServer {
    pub username:  *const c_char,
    pub password:  *const c_char,
    pub urls:      *const *const c_char,
    pub urls_size: usize,
}

#[derive(Clone, Debug)]
pub struct IceServer {
    username: CString,
    password: CString,
    // To own the strings
    urls:     Vec<CString>,
    // To hand the strings to C
    url_ptrs: Vec<*const c_char>,
}

unsafe impl Send for IceServer {}
unsafe impl Sync for IceServer {}

impl IceServer {
    pub fn new(username: String, password: String, urls_in: Vec<String>) -> Self {
        let mut urls = Vec::new();
        for url in urls_in {
            urls.push(CString::new(url).expect("CString of URL"));
        }
        let url_ptrs = urls.iter().map(|s| s.as_ptr()).collect();
        Self {
            username: CString::new(username).expect("CString of username"),
            password: CString::new(password).expect("CString of password"),
            urls,
            url_ptrs,
        }
    }

    pub fn rffi(&self) -> RffiIceServer {
        RffiIceServer {
            username:  self.username.as_ptr(),
            password:  self.password.as_ptr(),
            urls:      self.url_ptrs.as_ptr(),
            urls_size: self.url_ptrs.len(),
        }
    }
}

/// Describes an audio input or output device.
#[derive(Clone, Debug)]
pub struct AudioDevice {
    /// Name of the device
    pub name:            String,
    /// Unique ID - only available on Windows; empty string on all other platforms.
    pub unique_id:       String,
    /// Which of multiple devices with the same name this is.
    pub same_name_index: u16,
}

/// Rust wrapper around WebRTC C++ PeerConnectionFactoryInterface object.
pub struct PeerConnectionFactory {
    /// Pointer to C++ PeerConnectionFactoryInterface.
    rffi: *const pcf::RffiPeerConnectionFactoryInterface,
}

impl fmt::Display for PeerConnectionFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PeerConnectionFactory: {:p}", self.rffi)
    }
}

impl fmt::Debug for PeerConnectionFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Drop for PeerConnectionFactory {
    fn drop(&mut self) {
        debug!("PeerConnectionFactory::drop()");
        if !self.rffi.is_null() {
            ref_count::release_ref(self.rffi as CppObject);
        }
    }
}

unsafe impl Send for PeerConnectionFactory {}
unsafe impl Sync for PeerConnectionFactory {}

impl PeerConnectionFactory {
    /// Create a new Rust PeerConnectionFactory object from a WebRTC C++
    /// PeerConnectionFactoryInterface object.
    pub fn new(use_injectable_network: bool) -> Result<Self> {
        debug!("PeerConnectionFactory::new()");
        let rffi = unsafe { pcf::Rust_createPeerConnectionFactory(use_injectable_network) };
        if rffi.is_null() {
            return Err(RingRtcError::CreatePeerConnectionFactory.into());
        }
        Ok(Self { rffi })
    }

    #[cfg(feature = "simnet")]
    pub fn injectable_network(&self) -> Option<InjectableNetwork> {
        let rffi = unsafe { pcf::Rust_getInjectableNetwork(self.rffi) };
        if rffi.is_null() {
            return None;
        }
        Some(InjectableNetwork::new(rffi, self))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_peer_connection<T: Platform>(
        &self,
        observer: PeerConnectionObserver<T>,
        certificate: Certificate,
        hide_ip: bool,
        ice_servers: &IceServer,
        outgoing_audio: AudioTrack,
        outgoing_video: VideoSource,
        enable_dtls: bool,
        enable_rtp_data_channel: bool,
    ) -> Result<PeerConnection> {
        debug!(
            "PeerConnectionFactory::create_peer_connection() {}",
            self.rffi as u64
        );
        let rffi = unsafe {
            pcf::Rust_createPeerConnection(
                self.rffi,
                observer.rffi_interface(),
                certificate.rffi(),
                hide_ip,
                ice_servers.rffi(),
                outgoing_audio.rffi(),
                outgoing_video.rffi(),
                enable_dtls,
                enable_rtp_data_channel,
            )
        };
        debug!(
            "PeerConnectionFactory::create_peer_connection() finished: {}",
            rffi as u64
        );
        if rffi.is_null() {
            return Err(RingRtcError::CreatePeerConnection.into());
        }
        Ok(PeerConnection::owned(rffi))
    }

    pub fn create_outgoing_audio_track(&self) -> Result<AudioTrack> {
        debug!("PeerConnectionFactory::create_audio_track()");
        let rffi = unsafe { pcf::Rust_createAudioTrack(self.rffi) };
        if rffi.is_null() {
            return Err(RingRtcError::CreateAudioTrack.into());
        }
        Ok(AudioTrack::new(rffi))
    }

    pub fn create_outgoing_video_source(&self) -> Result<VideoSource> {
        debug!("PeerConnectionFactory::create_video_source()");
        let rffi = unsafe { pcf::Rust_createVideoSource(self.rffi) };
        if rffi.is_null() {
            return Err(RingRtcError::CreateVideoSource.into());
        }
        Ok(VideoSource::new(rffi))
    }

    pub fn get_audio_playout_devices(&self) -> Result<Vec<AudioDevice>> {
        debug!("PeerConnectionFactory::get_audio_playout_devices");
        let device_count = unsafe { pcf::Rust_getAudioPlayoutDevices(self.rffi) };
        let mut devices = Vec::<AudioDevice>::new();

        if device_count < 0 {
            return Err(RingRtcError::QueryAudioDevices.into());
        }
        let device_count = device_count as u16;
        for i in 0..device_count {
            let (name, unique_id, rc) = unsafe {
                let name =
                    CString::from_vec_unchecked(vec![0u8; ADM_MAX_DEVICE_NAME_SIZE]).into_raw();
                let unique_id =
                    CString::from_vec_unchecked(vec![0u8; ADM_MAX_DEVICE_UUID_SIZE]).into_raw();
                let rc = pcf::Rust_getAudioPlayoutDeviceName(self.rffi, i, name, unique_id);
                // Take back ownership of the raw pointers before checking for errors.
                let name = CString::from_raw(name);
                let unique_id = CString::from_raw(unique_id);
                (name, unique_id, rc)
            };
            if rc != 0 {
                return Err(RingRtcError::QueryAudioDevices.into());
            }
            let name = name.into_string()?;
            let unique_id = unique_id.into_string()?;
            let same_name_index = devices.iter().filter(|d| d.name == name).count() as u16;
            devices.push(AudioDevice {
                name,
                unique_id,
                same_name_index,
            })
        }
        Ok(devices)
    }

    pub fn set_audio_playout_device(&self, index: u16) -> Result<()> {
        info!(
            "PeerConnectionFactory::set_audio_playout_device({:?})",
            index
        );
        let ok = unsafe { pcf::Rust_setAudioPlayoutDevice(self.rffi, index) };
        match ok {
            true => Ok(()),
            false => Err(RingRtcError::SetAudioDevice.into()),
        }
    }

    pub fn get_audio_recording_devices(&self) -> Result<Vec<AudioDevice>> {
        debug!("PeerConnectionFactory::get_audio_recording_devices");
        let device_count = unsafe { pcf::Rust_getAudioRecordingDevices(self.rffi) };
        let mut devices = Vec::<AudioDevice>::new();

        if device_count < 0 {
            return Err(RingRtcError::QueryAudioDevices.into());
        }
        let device_count = device_count as u16;

        for i in 0..device_count {
            let (name, unique_id, rc) = unsafe {
                let name =
                    CString::from_vec_unchecked(vec![0u8; ADM_MAX_DEVICE_NAME_SIZE]).into_raw();
                let unique_id =
                    CString::from_vec_unchecked(vec![0u8; ADM_MAX_DEVICE_UUID_SIZE]).into_raw();
                let rc = pcf::Rust_getAudioRecordingDeviceName(self.rffi, i, name, unique_id);
                // Take back ownership of the raw pointers before checking for errors.
                let name = CString::from_raw(name);
                let unique_id = CString::from_raw(unique_id);
                (name, unique_id, rc)
            };
            if rc != 0 {
                return Err(RingRtcError::QueryAudioDevices.into());
            }
            let name = name.into_string()?;
            let unique_id = unique_id.into_string()?;
            let same_name_index = devices.iter().filter(|d| d.name == name).count() as u16;
            devices.push(AudioDevice {
                name,
                unique_id,
                same_name_index,
            })
        }
        Ok(devices)
    }

    pub fn set_audio_recording_device(&self, index: u16) -> Result<()> {
        info!(
            "PeerConnectionFactory::set_audio_recording_device({:?})",
            index
        );
        let ok = unsafe { pcf::Rust_setAudioRecordingDevice(self.rffi, index) };
        match ok {
            true => Ok(()),
            false => Err(RingRtcError::SetAudioDevice.into()),
        }
    }
}

impl Clone for PeerConnectionFactory {
    fn clone(&self) -> Self {
        info!("PeerConnectionFactory::clone() {}", self.rffi as u64);
        if !self.rffi.is_null() {
            ref_count::add_ref(self.rffi as CppObject);
        }
        Self { rffi: self.rffi }
    }
}
