<div align="center">
  <img src="./apps/app-frontend/src/assets/about/yudream-launcher-banner.jpg" alt="YMCL (YuDream Launcher)" />
  <h1>YMCL (YuDream Launcher)</h1>
  <p><strong>一款专为to B设计的启动器，由YuDream Admin Skin驱动。</strong></p>

  <p>
    <a href="https://github.com/mcyudream/YMCL-Axolotl/actions">
      <img src="https://img.shields.io/github/actions/workflow/status/mcyudream/YMCL-Axolotl/axolotl-ci.yml?style=for-the-badge&logo=github" alt="Desktop CI" />
    </a>
    <a href="https://github.com/mcyudream/YMCL-Axolotl/releases">
      <img src="https://img.shields.io/github/downloads/mcyudream/YMCL-Axolotl/total?style=for-the-badge&logo=github" alt="Downloads" />
    </a>
    <a href="https://github.com/mcyudream/YMCL-Axolotl/stargazers">
      <img src="https://img.shields.io/github/stars/mcyudream/YMCL-Axolotl?style=for-the-badge&logo=github&color=ffb800" alt="Stars" />
    </a>
    <a href="COPYING.md">
      <img src="https://img.shields.io/badge/License-GPL_3.0-blue.svg?style=for-the-badge" alt="License" />
    </a>
  </p>

  <p>
    <a href="https://github.com/mcyudream/YMCL-Axolotl/releases/latest">下载最新版</a> ｜
    <a href="https://github.com/mcyudream/YMCL-Axolotl">源代码</a> ｜
    <a href="https://github.com/mcyudream/YMCL-Axolotl/issues">问题与反馈</a>
  </p>

  <p>
    <a href="CONTRIBUTING.md">参与贡献</a> ｜
    <a href="CODE_OF_CONDUCT.md">行为准则</a>
  </p>
</div>

---

**YMCL (YuDream Launcher)** 是基于开源启动器 Axolotl（美西螈）二次开发的开源 Minecraft 启动器。它是一款免费、开源、跨平台的 Minecraft Java 版第三方启动器，支持在一个客户端中搜索、安装和更新来自 Modrinth 与 CurseForge 的模组、整合包、资源包和光影，并提供实例管理、多种账户认证、个性化外观与实验室工具。

上游基于 [Modrinth App](https://github.com/modrinth/code) 构建，移除了不适用于本项目的商业化模块，专注于提供纯净、无广告的桌面启动体验。

本项目与客户端项目 Axolotl Client 无任何关联。

_Modrinth 是 Rinth, Inc. 的商标。YMCL (YuDream Launcher) 与 Rinth, Inc. 无关联，亦未获得其认可。_

## 核心优势

- **域自定义**：主题与首页卡片布局随域下发，管理员与获授权成员在启动器内即可完成界面设计，并一键发布到整个域，全体成员即时生效，无需重新打包分发客户端。
- **域热页面**：域插件以模块化页面的形式向启动器注入功能页面，服务端发布即热加载生效，功能上新不再受客户端发版节奏限制。
- **整合包发布与权威更新**：整合包绑定服务器后即可发布，玩家端在启动前自动完成权威更新，全服版本始终一致，玩家全程无感。
- **域账户统一认证**：对接 yggdrasil / authlib-injector 外置登录，域会话自动续期、过期自动唤起重新登录，玩家一次登录长期畅玩。
- **资源镜像加速**：GitHub 侧资源发布时自动入域内容镜像（CAS），安装时镜像优先下载，国内网络环境下模组与整合包获取依然顺畅。

在此基础上，YMCL 仍保留完整的通用体验：原生支持 Windows、macOS 与 Linux，集成 Modrinth 与 CurseForge 内容生态一键安装与更新，并内置「实验室」工具箱。

## 下载与安装

请前往 [GitHub Releases](https://github.com/mcyudream/YMCL-Axolotl/releases/latest) 下载适合你操作系统的最新安装包。
已安装的用户每次均可通过内置的 Tauri 签名校验机制，自动在后台完成更新，无需手动下载安装更新。

| 系统平台                | 推荐下载文件                              |
| ----------------------- | ----------------------------------------- |
| **Windows** (10/11 x64) | 下载 `.exe` (NSIS) 安装程序               |
| **macOS**               | 下载 `通用 .dmg` 镜像文件                 |
| **Linux** (x64)         | 提供 `.AppImage`，`.deb`，`.rpm` 多种格式 |

## 开发组

<a href="https://github.com/YDHusky"><img src="https://github.com/YDHusky.png?size=96" width="48" height="48" alt="SiberianHusky" title="SiberianHusky" /></a>

- [SiberianHusky](https://github.com/YDHusky)

## 项目与社区

| 入口 | 说明 |
| ---- | ---- |
| [源代码](https://github.com/mcyudream/YMCL-Axolotl) | 仓库与开发进展 |
| [问题与反馈](https://github.com/mcyudream/YMCL-Axolotl/issues) | Bug 报告与功能建议 |
| [在爱发电赞助](https://afdian.com/a/Mystic-Stars) | 支持项目持续开发 |

## 贡献者

与 `apps/app-frontend/src/data/about/contributors.json` 对齐（当前仓库贡献者，不含上游）：

- [SiberianHusky](https://github.com/YDHusky)

完整名单可在应用内「设置 → 关于」查看，也可访问 [GitHub Contributors](https://github.com/mcyudream/YMCL-Axolotl/graphs/contributors)。

## 许可证与来源声明

YMCL (YuDream Launcher) 是基于开源启动器 Axolotl（美西螈）二次开发的开源 Minecraft 启动器。

- [基于美西螈（Axolotl）二次开发](https://github.com/mcyudream/YMCL-Axolotl)
- [项目许可证（GPL-3.0）](COPYING.md)
- [复制准则（Copying guidelines）](COPYING.md)
- [第三方许可证](third-party/licenses)
- [Modrinth 原始源代码](https://github.com/modrinth/code)

Modrinth 是 Rinth, Inc. 的商标。YMCL (YuDream Launcher) 与 Rinth, Inc. 无关联，亦未获得其认可。

## 参与项目开发

YMCL (YuDream Launcher) 的进步离不开社区的反馈与贡献。
如果遇到 Bug 或有新的功能点子，欢迎提交 [Issue](https://github.com/mcyudream/YMCL-Axolotl/issues)。如需搭建本地开发环境或查阅打包发布规范，请阅读详细的 [贡献指南 (CONTRIBUTING.md)](CONTRIBUTING.md)。
参与社区和贡献代码前，也请先阅读[行为准则 (CODE_OF_CONDUCT.md)](CODE_OF_CONDUCT.md)。
