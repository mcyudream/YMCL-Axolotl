/** Keep transport diagnostics in details rather than using them as landing-page copy. */
export function domainHomeErrorKind(message: string) {
	if (
		/session expired|unauthorized|forbidden|\b(?:401|403)\b|会话.*过期|未登录|无权|权限不足/i.test(
			message,
		)
	)
		return 'errorAuth'
	if (
		/no space left|disk full|not enough (?:disk )?space|磁盘空间不足|空间不足|os error 112/i.test(
			message,
		)
	)
		return 'errorSpace'
	if (
		/network|connection|timed?\s*out|timeout|broken pipe|dns|response body|error sending request|网络|连接|超时/i.test(
			message,
		)
	)
		return 'errorNetwork'
	return 'errorGeneric'
}
