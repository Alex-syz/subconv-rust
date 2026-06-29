//! Integration tests for all 14 protocol parsers.

use subconv::converter::proxy::Proxy;
use subconv::converter::registry::NameRegistry;
use subconv::converter::{
    anytls, hysteria, hysteria2, http_socks, mieru, ss, ssr, telegram, trojan, tuic,
    vless, vmess, converts_v2ray,
};

#[test]
fn ss_standard_format() {
    let mut reg = NameRegistry::new();
    let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@server:443#TestSS";
    let result = ss::parse_ss(uri, &mut reg).unwrap();
    if let Proxy::Ss(p) = result {
        assert_eq!(p.cipher, "aes-256-gcm");
        assert_eq!(p.password, "password");
        assert_eq!(p.server, "server");
        assert_eq!(p.port, 443);
        assert_eq!(p.name, "TestSS");
    } else {
        panic!("Expected Ss proxy");
    }
}

#[test]
fn ss_with_plugin() {
    let mut reg = NameRegistry::new();
    let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@server:443?plugin=obfs-local%3Bobfs%3Dhttp%3Bobfs-host%3Dexample.com#WithPlugin";
    let result = ss::parse_ss(uri, &mut reg).unwrap();
    if let Proxy::Ss(p) = result {
        assert_eq!(p.plugin, Some("obfs".to_string()));
        assert!(p.plugin_opts.is_some());
    } else {
        panic!("Expected Ss proxy");
    }
}

#[test]
fn ssr_basic() {
    let mut reg = NameRegistry::new();
    use base64::Engine;
    let inner = "server:8388:auth_aes128_md5:aes-256-cfb:tls:aGVsbG8=?obfsparam=&protoparam=&remarks=VGVzdA";
    let encoded = base64::engine::general_purpose::STANDARD.encode(inner);
    let uri = format!("ssr://{encoded}");
    let result = ssr::parse_ssr(&uri, &mut reg).unwrap();
    if let Proxy::Ssr(p) = result {
        assert_eq!(p.server, "server");
        assert_eq!(p.port, 8388);
        assert_eq!(p.cipher, "aes-256-cfb");
        assert_eq!(p.password, "hello");
        assert_eq!(p.name, "Test");
    } else {
        panic!("Expected Ssr proxy");
    }
}

#[test]
fn vmess_legacy_format() {
    let mut reg = NameRegistry::new();
    use base64::Engine;
    let json = r#"{"v":"2","ps":"TestVMess","add":"server","port":"443","id":"12345678-1234-1234-1234-123456789abc","aid":"0","scy":"auto","net":"ws","type":"none","host":"","path":"/ws","tls":"tls"}"#;
    let encoded = base64::engine::general_purpose::STANDARD.encode(json);
    let uri = format!("vmess://{encoded}");
    let result = vmess::parse_vmess(&uri, &mut reg).unwrap();
    if let Proxy::Vmess(p) = result {
        assert_eq!(p.server, "server");
        assert_eq!(p.port, 443);
        assert_eq!(p.uuid, "12345678-1234-1234-1234-123456789abc");
        assert!(p.tls);
        assert_eq!(p.network, Some("ws".to_string()));
    } else {
        panic!("Expected Vmess proxy");
    }
}

#[test]
fn vless_basic() {
    let mut reg = NameRegistry::new();
    let uri = "vless://12345678-1234-1234-1234-123456789abc@example.com:443?type=ws&security=tls&sni=example.com#TestVless";
    let result = vless::parse_vless(uri, &mut reg).unwrap();
    if let Proxy::Vless(p) = result {
        assert_eq!(p.uuid, "12345678-1234-1234-1234-123456789abc");
        assert_eq!(p.server, "example.com");
        assert_eq!(p.tls, Some(true));
        assert_eq!(p.servername, Some("example.com".to_string()));
    } else {
        panic!("Expected Vless proxy");
    }
}

#[test]
fn vless_reality() {
    let mut reg = NameRegistry::new();
    let uri = "vless://uuid@server:443?security=reality&pbk=pubkey&sid=shortid&fp=chrome&type=tcp#RealityTest";
    let result = vless::parse_vless(uri, &mut reg).unwrap();
    if let Proxy::Vless(p) = result {
        assert_eq!(p.tls, Some(true));
        let ro = p.reality_opts.unwrap();
        assert_eq!(ro.public_key, "pubkey");
        assert_eq!(ro.short_id, Some("shortid".to_string()));
    } else {
        panic!("Expected Vless proxy");
    }
}

#[test]
fn trojan_basic() {
    let mut reg = NameRegistry::new();
    let uri = "trojan://password123@server:443?sni=example.com&type=ws&path=/ws#TestTrojan";
    let result = trojan::parse_trojan(uri, &mut reg).unwrap();
    if let Proxy::Trojan(p) = result {
        assert_eq!(p.password, "password123");
        assert_eq!(p.server, "server");
        assert_eq!(p.sni, Some("example.com".to_string()));
        assert_eq!(p.network, Some("ws".to_string()));
    } else {
        panic!("Expected Trojan proxy");
    }
}

#[test]
fn trojan_grpc() {
    let mut reg = NameRegistry::new();
    let uri = "trojan://pass@server:443?type=grpc&serviceName=myservice#GrpcTrojan";
    let result = trojan::parse_trojan(uri, &mut reg).unwrap();
    if let Proxy::Trojan(p) = result {
        assert_eq!(p.network, Some("grpc".to_string()));
    } else {
        panic!("Expected Trojan proxy");
    }
}

#[test]
fn hysteria_basic() {
    let mut reg = NameRegistry::new();
    let uri = "hysteria://server:443?peer=sni.example.com&auth=myauth&upmbps=100&downmbps=200&obfs=salamander#TestHysteria";
    let result = hysteria::parse_hysteria(uri, &mut reg).unwrap();
    if let Proxy::Hysteria(p) = result {
        assert_eq!(p.server, "server");
        assert_eq!(p.sni, Some("sni.example.com".to_string()));
        assert_eq!(p.auth_str, Some("myauth".to_string()));
        assert_eq!(p.up, Some("100".to_string()));
    } else {
        panic!("Expected Hysteria proxy");
    }
}

#[test]
fn hysteria2_basic() {
    let mut reg = NameRegistry::new();
    let uri = "hysteria2://authpassword@server:443?sni=example.com&obfs=salamander&obfs-password=obfspass#TestHy2";
    let result = hysteria2::parse_hysteria2(uri, &mut reg).unwrap();
    if let Proxy::Hysteria2(p) = result {
        assert_eq!(p.password, Some("authpassword".to_string()));
        assert_eq!(p.sni, Some("example.com".to_string()));
        assert_eq!(p.obfs, Some("salamander".to_string()));
    } else {
        panic!("Expected Hysteria2 proxy");
    }
}

#[test]
fn hysteria2_default_port() {
    let mut reg = NameRegistry::new();
    let uri = "hysteria2://auth@server?sni=example.com#NoPort";
    let result = hysteria2::parse_hysteria2(uri, &mut reg).unwrap();
    if let Proxy::Hysteria2(p) = result {
        assert_eq!(p.port, 443);
    } else {
        panic!("Expected Hysteria2 proxy");
    }
}

#[test]
fn tuic_v5() {
    let mut reg = NameRegistry::new();
    let uri = "tuic://my-uuid:my-password@server:8443?sni=example.com&congestion_control=cubic#TestTuic";
    let result = tuic::parse_tuic(uri, &mut reg).unwrap();
    if let Proxy::Tuic(p) = result {
        assert_eq!(p.uuid, Some("my-uuid".to_string()));
        assert_eq!(p.password, Some("my-password".to_string()));
        assert_eq!(p.token, None);
        assert_eq!(p.congestion_controller, Some("cubic".to_string()));
    } else {
        panic!("Expected Tuic proxy");
    }
}

#[test]
fn tuic_v4() {
    let mut reg = NameRegistry::new();
    let uri = "tuic://my-token@server:8443?sni=example.com#TuicV4";
    let result = tuic::parse_tuic(uri, &mut reg).unwrap();
    if let Proxy::Tuic(p) = result {
        assert_eq!(p.token, Some("my-token".to_string()));
    } else {
        panic!("Expected Tuic proxy");
    }
}

#[test]
fn http_basic() {
    let mut reg = NameRegistry::new();
    let uri = "http://user:pass@server:8080#TestHTTP";
    let result = http_socks::parse_http(uri, &mut reg).unwrap();
    if let Proxy::Http(p) = result {
        assert_eq!(p.server, "server");
        assert_eq!(p.port, 8080);
        assert_eq!(p.username, Some("user".to_string()));
        assert_eq!(p.password, Some("pass".to_string()));
    } else {
        panic!("Expected Http proxy");
    }
}

#[test]
fn https_default_port() {
    let mut reg = NameRegistry::new();
    let uri = "https://user:pass@server#TestHTTPS";
    let result = http_socks::parse_http(uri, &mut reg).unwrap();
    if let Proxy::Http(p) = result {
        assert_eq!(p.tls, Some(true));
        assert_eq!(p.port, 443);
    } else {
        panic!("Expected Http proxy");
    }
}

#[test]
fn socks5_basic() {
    let mut reg = NameRegistry::new();
    let uri = "socks5://user:pass@server:1080#TestSocks5";
    let result = http_socks::parse_socks5(uri, &mut reg).unwrap();
    if let Proxy::Socks5(p) = result {
        assert_eq!(p.server, "server");
        assert_eq!(p.port, 1080);
    } else {
        panic!("Expected Socks5 proxy");
    }
}

#[test]
fn telegram_tg() {
    let mut reg = NameRegistry::new();
    let uri = "tg://proxy?server=1.2.3.4&port=1080&user=secret&pass=secret2#TestTG";
    let result = telegram::parse_telegram(uri, &mut reg).unwrap();
    if let Proxy::Telegram(p) = result {
        assert_eq!(p.server, "1.2.3.4");
        assert_eq!(p.port, 1080);
        assert_eq!(p.username, Some("secret".to_string()));
    } else {
        panic!("Expected Telegram proxy");
    }
}

#[test]
fn telegram_https() {
    let mut reg = NameRegistry::new();
    let uri = "https://t.me/proxy?server=1.2.3.4&port=1080&user=secret&pass=secret2";
    let result = telegram::parse_telegram_https(uri, &mut reg).unwrap();
    if let Proxy::Telegram(p) = result {
        assert_eq!(p.server, "1.2.3.4");
        assert_eq!(p.port, 1080);
    } else {
        panic!("Expected Telegram proxy");
    }
}

#[test]
fn anytls_basic() {
    let mut reg = NameRegistry::new();
    let uri = "anytls://user:pass@server:443?sni=example.com&hpkp=sha256/abc&insecure=1#TestAnyTLS";
    let result = anytls::parse_anytls(uri, &mut reg).unwrap();
    if let Proxy::Anytls(p) = result {
        assert_eq!(p.password, "pass");
        assert_eq!(p.sni, Some("example.com".to_string()));
        assert!(p.skip_cert_verify);
    } else {
        panic!("Expected Anytls proxy");
    }
}

#[test]
fn anytls_password_fallback() {
    let mut reg = NameRegistry::new();
    let uri = "anytls://onlyuser@server:443#Fallback";
    let result = anytls::parse_anytls(uri, &mut reg).unwrap();
    if let Proxy::Anytls(p) = result {
        assert_eq!(p.password, "onlyuser");
    } else {
        panic!("Expected Anytls proxy");
    }
}

#[test]
fn mieru_basic() {
    let mut reg = NameRegistry::new();
    let uri = "mierus://user:pass@server?port=8964&protocol=tcp#TestMieru";
    let result = mieru::parse_mieru(uri, &mut reg).unwrap();
    assert_eq!(result.len(), 1);
    if let Proxy::Mieru(p) = &result[0] {
        assert_eq!(p.port, Some(8964));
        assert_eq!(p.transport, "tcp");
    } else {
        panic!("Expected Mieru proxy");
    }
}

#[test]
fn mieru_multiple_ports() {
    let mut reg = NameRegistry::new();
    let uri = "mierus://server?port=8964&port=8965&protocol=tcp&protocol=udp#MultiPort";
    let result = mieru::parse_mieru(uri, &mut reg).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn mieru_port_range() {
    let mut reg = NameRegistry::new();
    let uri = "mierus://server?port=8964-8970&protocol=tcp#PortRange";
    let result = mieru::parse_mieru(uri, &mut reg).unwrap();
    if let Proxy::Mieru(p) = &result[0] {
        assert_eq!(p.port, None);
        assert_eq!(p.port_range, Some("8964-8970".to_string()));
    } else {
        panic!("Expected Mieru proxy");
    }
}

#[test]
fn converts_v2ray_trojan() {
    let mut reg = NameRegistry::new();
    let input = "trojan://password123@server:443?sni=example.com#TestTrojan";
    let result = converts_v2ray(input, &mut reg).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0], Proxy::Trojan(_)));
}

#[test]
fn converts_v2ray_skips_malformed() {
    let mut reg = NameRegistry::new();
    let input = "trojan://password123@server:443?sni=example.com#Good\ninvalid-line";
    let result = converts_v2ray(input, &mut reg).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn converts_v2ray_base64_encoded() {
    let mut reg = NameRegistry::new();
    use base64::Engine;
    let plain = "trojan://password123@server:443?sni=example.com#B64Test";
    let encoded = base64::engine::general_purpose::STANDARD.encode(plain);
    let result = converts_v2ray(&encoded, &mut reg).unwrap();
    assert_eq!(result.len(), 1);
}
