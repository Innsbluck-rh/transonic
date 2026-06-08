package version

const ProtocolVersion = 2

var (
	Version   = "0.0.0-dev"
	Commit    = "unknown"
	BuildDate = "unknown"
)

type Info struct {
	Version         string `json:"version"`
	Commit          string `json:"commit"`
	BuildDate       string `json:"buildDate"`
	ProtocolVersion int    `json:"protocolVersion"`
}

func Current() Info {
	return Info{
		Version:         Version,
		Commit:          Commit,
		BuildDate:       BuildDate,
		ProtocolVersion: ProtocolVersion,
	}
}
